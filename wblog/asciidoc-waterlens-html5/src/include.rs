//! File-backed resolution of `include::` directives.
//!
//! `asciidoc-parser` deliberately does not touch the filesystem itself: an
//! `include::` directive is handed to an [`IncludeFileHandler`], and with none
//! configured every include renders as "Unresolved directive". This module
//! supplies the handler the site needs — one that reads the included file
//! relative to the including document, the way Asciidoctor does.
//!
//! The parser reports the *including* file through `resolve_target`'s
//! `source` parameter: the primary file for the outermost document (the
//! `Options::input_file`), and — for a directive inside an include — the raw
//! `target` string of the enclosing directive (see the parser's
//! preprocessor). The handler therefore records, for every target it has
//! resolved, the directory the resolved file lives in, and looks that up when
//! the same string arrives as a `source`.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use asciidoc_parser::{
    Parser, SafeMode,
    attributes::Attrlist,
    parser::{IncludeContent, IncludeFileHandler, IncludeResolution},
};

use crate::path::{FileReadError, FileResolver};

/// The `IncludeFileHandler` the backend installs when a primary input file is
/// known: it resolves each directive's target against the directory of the
/// file containing the directive and reads the result from disk.
#[derive(Debug)]
pub(crate) struct FileIncludeHandler {
    /// Resolves paths against the primary file's directory and enforces the
    /// selected safe mode.
    files: FileResolver,

    /// For each raw include target that has been resolved, the directory of
    /// the file it resolved to — so a nested `include::` inside that file can
    /// resolve *its* target relative to it. The parser hands nested includes
    /// the enclosing directive's raw target string as their `source`, so the
    /// map is keyed by that string.
    ///
    /// Interior mutability is required because the trait's `resolve_target`
    /// takes `&self` (the handler sits behind an `Rc` in the parser).
    resolved_dirs: RefCell<HashMap<String, PathBuf>>,
}

impl FileIncludeHandler {
    /// A handler anchored at `input_file`'s directory. When `input_file` has
    /// no parent (a bare file name), the anchor is the empty path, which
    /// resolves includes relative to the process's current directory.
    pub(crate) fn new(input_file: &Path, safe_mode: SafeMode) -> Self {
        Self {
            files: FileResolver::for_input_file(input_file, safe_mode),
            resolved_dirs: RefCell::new(HashMap::new()),
        }
    }
}

impl IncludeFileHandler for FileIncludeHandler {
    fn resolve_target<'src>(
        &self,
        source: Option<&str>,
        target: &str,
        _attrlist: &Attrlist<'src>,
        _parser: &Parser,
    ) -> IncludeResolution {
        // The directory to resolve `target` against: the enclosing file's
        // directory when we have seen it, the primary file's directory
        // otherwise.
        let current_dir = match source {
            Some(source) => self
                .resolved_dirs
                .borrow()
                .get(source)
                .cloned()
                .unwrap_or_else(|| self.files.base_dir().to_path_buf()),
            None => self.files.base_dir().to_path_buf(),
        };

        let path = self.files.resolve(Some(&current_dir), target);

        match self.files.read(Some(&current_dir), target) {
            Ok(mut contents) => {
                // Remember where this file lives so an include *inside* it
                // resolves relative to it.
                if let Some(parent) = path.parent() {
                    self.resolved_dirs
                        .borrow_mut()
                        .insert(target.to_string(), parent.to_path_buf());
                }
                if contents.starts_with('\u{feff}') {
                    contents.drain(..'\u{feff}'.len_utf8());
                }
                IncludeResolution::Found(IncludeContent::new(contents))
            }
            Err(FileReadError::Missing) => IncludeResolution::NotFound,
            Err(FileReadError::Unreadable) => IncludeResolution::NotReadable,
            Err(FileReadError::InvalidUtf8) => IncludeResolution::NotDecodable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileIncludeHandler;
    use asciidoc_parser::{
        Parser, SafeMode,
        parser::{IncludeFileHandler, IncludeResolution},
    };

    #[test]
    fn resolves_a_relative_include_from_the_primary_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("header.adoc"), "= Included").expect("write");
        let handler = FileIncludeHandler::new(&dir.path().join("main.adoc"), SafeMode::Unsafe);

        let resolution =
            handler.resolve_target(None, "header.adoc", &Default::default(), &Parser::default());
        match resolution {
            IncludeResolution::Found(content) => assert_eq!(content.content(), "= Included"),
            other => panic!("expected found, got {other:?}"),
        }
    }

    #[test]
    fn resolves_a_nested_include_relative_to_the_enclosing_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("sub")).expect("mkdir");
        std::fs::write(dir.path().join("sub/outer.adoc"), "outer").expect("write");
        std::fs::write(dir.path().join("sub/inner.adoc"), "inner").expect("write");
        let handler = FileIncludeHandler::new(&dir.path().join("main.adoc"), SafeMode::Unsafe);

        // First the outer include is resolved and recorded...
        assert!(matches!(
            handler.resolve_target(
                None,
                "sub/outer.adoc",
                &Default::default(),
                &Parser::default()
            ),
            IncludeResolution::Found(_)
        ));
        // ...then an include *inside* it arrives with the raw target as its
        // `source` and resolves against the outer file's directory.
        let resolution = handler.resolve_target(
            Some("sub/outer.adoc"),
            "inner.adoc",
            &Default::default(),
            &Parser::default(),
        );
        match resolution {
            IncludeResolution::Found(content) => assert_eq!(content.content(), "inner"),
            other => panic!("expected found, got {other:?}"),
        }
    }

    #[test]
    fn missing_files_report_not_found() {
        let dir = tempfile::tempdir().expect("temp dir");
        let handler = FileIncludeHandler::new(&dir.path().join("main.adoc"), SafeMode::Unsafe);
        assert_eq!(
            handler.resolve_target(None, "nope.adoc", &Default::default(), &Parser::default()),
            IncludeResolution::NotFound
        );
    }

    #[test]
    fn an_absolute_target_ignores_the_anchor() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("abs.adoc"), "absolute").expect("write");
        let handler =
            FileIncludeHandler::new(&dir.path().join("other/main.adoc"), SafeMode::Unsafe);
        let resolution = handler.resolve_target(
            None,
            &dir.path().join("abs.adoc").to_string_lossy(),
            &Default::default(),
            &Parser::default(),
        );
        match resolution {
            IncludeResolution::Found(content) => assert_eq!(content.content(), "absolute"),
            other => panic!("expected found, got {other:?}"),
        }
    }
}
