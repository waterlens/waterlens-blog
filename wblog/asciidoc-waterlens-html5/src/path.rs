//! Filesystem and web-path resolution shared by includes and rendered assets.
//!
//! Filesystem paths and URLs deliberately use separate helpers. Document
//! attribute values are HTML-substituted by the parser, which is correct when
//! writing an `href` but not when opening a local file. Likewise, URL
//! normalization must preserve a scheme's `//`, while filesystem resolution
//! must enforce Asciidoctor's safe-mode jail.

use std::{
    env, fs, io,
    path::{Component, Path, PathBuf},
};

use asciidoc_parser::SafeMode;

/// Why a UTF-8 text file could not be read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileReadError {
    Missing,
    Unreadable,
    InvalidUtf8,
}

/// Resolves and reads local files relative to one document base directory.
#[derive(Clone, Debug)]
pub(crate) struct FileResolver {
    base_dir: PathBuf,
    safe_mode: SafeMode,
}

impl FileResolver {
    /// Creates a resolver rooted at `base_dir`.
    pub(crate) fn new(base_dir: &Path, safe_mode: SafeMode) -> Self {
        let base_dir = absolute_normalized(base_dir);
        let base_dir = base_dir.canonicalize().unwrap_or(base_dir);
        Self {
            base_dir,
            safe_mode,
        }
    }

    /// Creates a resolver rooted at the primary input file's directory.
    pub(crate) fn for_input_file(input_file: &Path, safe_mode: SafeMode) -> Self {
        Self::new(
            input_file.parent().unwrap_or_else(|| Path::new("")),
            safe_mode,
        )
    }

    pub(crate) fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub(crate) fn safe_mode(&self) -> SafeMode {
        self.safe_mode
    }

    /// Resolves a document-supplied starting directory against the base.
    ///
    /// In a jailed mode an absolute directory is honored only when it is
    /// already inside the jail; otherwise resolution recovers to the base.
    pub(crate) fn resolve_directory(&self, directory: &str) -> PathBuf {
        let path = Path::new(directory);
        if self.safe_mode >= SafeMode::Safe && path.is_absolute() {
            return path.strip_prefix(&self.base_dir).map_or_else(
                |_| self.base_dir.clone(),
                |relative| self.base_dir.join(relative),
            );
        }

        self.resolve(None, directory)
    }

    /// Resolves `target` relative to `current_dir` (or the document base).
    pub(crate) fn resolve(&self, current_dir: Option<&Path>, target: &str) -> PathBuf {
        let current_dir = current_dir.unwrap_or(&self.base_dir);
        let target = Path::new(target);

        if self.safe_mode >= SafeMode::Safe {
            resolve_confined(&self.base_dir, current_dir, target)
        } else if target.is_absolute() {
            normalize(target)
        } else {
            normalize(&current_dir.join(target))
        }
    }

    /// Resolves and reads `target` as UTF-8 text.
    pub(crate) fn read(
        &self,
        current_dir: Option<&Path>,
        target: &str,
    ) -> Result<String, FileReadError> {
        let path = self.resolve(current_dir, target);
        let path = if self.safe_mode >= SafeMode::Safe {
            let real = path.canonicalize().map_err(classify_io_error)?;
            if !real.starts_with(&self.base_dir) {
                return Err(FileReadError::Missing);
            }
            real
        } else {
            path
        };

        let metadata = fs::metadata(&path).map_err(classify_io_error)?;
        if !metadata.is_file() {
            return Err(FileReadError::Missing);
        }

        let bytes = fs::read(path).map_err(classify_io_error)?;
        String::from_utf8(bytes).map_err(|_| FileReadError::InvalidUtf8)
    }
}

fn classify_io_error(error: io::Error) -> FileReadError {
    match error.kind() {
        io::ErrorKind::NotFound => FileReadError::Missing,
        _ => FileReadError::Unreadable,
    }
}

/// Reinterprets absolute targets inside `base_dir` and clamps `..` at it.
fn resolve_confined(base_dir: &Path, current_dir: &Path, target: &Path) -> PathBuf {
    let mut resolved = base_dir.to_path_buf();

    if !target.is_absolute()
        && let Ok(relative_current) = current_dir.strip_prefix(base_dir)
    {
        extend_confined(&mut resolved, base_dir, relative_current.components());
    }

    extend_confined(&mut resolved, base_dir, target.components());
    resolved
}

fn extend_confined<'a>(
    path: &mut PathBuf,
    base_dir: &Path,
    components: impl Iterator<Item = Component<'a>>,
) {
    for component in components {
        match component {
            Component::Normal(segment) => path.push(segment),
            Component::ParentDir if path != base_dir => {
                path.pop();
            }
            Component::ParentDir
            | Component::CurDir
            | Component::RootDir
            | Component::Prefix(_) => {}
        }
    }
}

fn absolute_normalized(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return normalize(path);
    }

    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    normalize(&current_dir.join(path))
}

/// Lexically removes `.` and resolves `..` without touching the filesystem.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                _ => normalized.push(component),
            },
            other => normalized.push(other),
        }
    }

    normalized
}

/// Reverses the parser's special-character substitution for filesystem use.
///
/// Decoding `&amp;` last preserves a literal entity written in the source:
/// `&amp;lt;` becomes `&lt;`, not `<`.
pub(crate) fn decode_document_attribute(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Whether `target` has an absolute URI scheme understood by the backend.
pub(crate) fn looks_like_uri(target: &str) -> bool {
    match target.find("://") {
        Some(index) if index > 0 => target[..index]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-')),
        _ => target.starts_with("data:") || target.starts_with("mailto:"),
    }
}

/// Resolves an asset target against its web directory and normalizes `.`/`..`.
pub(crate) fn resolve_web_path(target: &str, directory: &str) -> String {
    if looks_like_uri(target) {
        return target.replace(' ', "%20");
    }

    let target = target.replace('\\', "/");
    let directory = directory.replace('\\', "/");
    let joined = if directory.is_empty() || target.starts_with('/') {
        target
    } else {
        format!("{}/{}", directory.trim_end_matches('/'), target)
    };

    normalize_web_path(&joined).replace(' ', "%20")
}

/// Lexically normalizes a web path without corrupting URI authority markers.
fn normalize_web_path(path: &str) -> String {
    let (prefix, rest, rooted) = split_web_prefix(path);
    let mut segments: Vec<&str> = Vec::new();

    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => match segments.last() {
                Some(&last) if last != ".." => {
                    segments.pop();
                }
                _ if rooted => {}
                _ => segments.push(".."),
            },
            other => segments.push(other),
        }
    }

    format!("{prefix}{}", segments.join("/"))
}

/// Separates a URI authority, absolute-root marker, or leading `./` from the
/// path segments that may be normalized.
fn split_web_prefix(path: &str) -> (&str, &str, bool) {
    if let Some(scheme_end) = path.find("://") {
        let authority_start = scheme_end + 3;
        if let Some(path_start) = path[authority_start..].find('/') {
            let path_start = authority_start + path_start + 1;
            return (&path[..path_start], &path[path_start..], true);
        }
        return (path, "", true);
    }

    if let Some(rest) = path.strip_prefix("//") {
        if let Some(path_start) = rest.find('/') {
            let path_start = path_start + 3;
            return (&path[..path_start], &path[path_start..], true);
        }
        return (path, "", true);
    }

    if let Some(rest) = path.strip_prefix('/') {
        ("/", rest, true)
    } else if let Some(rest) = path.strip_prefix("./") {
        ("./", rest, false)
    } else {
        ("", path, false)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::{FileReadError, FileResolver, decode_document_attribute, resolve_web_path};
    use asciidoc_parser::SafeMode;

    #[test]
    fn unsafe_resolution_can_leave_the_document_base() {
        let root = tempfile::tempdir().expect("root dir");
        let base = root.path().join("a/b");
        std::fs::create_dir_all(&base).expect("create base dir");
        let resolver = FileResolver::new(&base, SafeMode::Unsafe);
        assert_eq!(
            resolver.resolve(None, "../../shared.adoc"),
            root.path()
                .canonicalize()
                .expect("canonical root")
                .join("shared.adoc")
        );
    }

    #[test]
    fn safe_resolution_clamps_absolute_and_parent_targets() {
        let base = tempfile::tempdir().expect("base dir");
        let resolver = FileResolver::new(base.path(), SafeMode::Safe);
        assert_eq!(
            resolver.resolve(None, "../../shared.adoc"),
            resolver.base_dir().join("shared.adoc")
        );

        let absolute = env::temp_dir().join("outside.adoc");
        let recovered = resolver.resolve(None, absolute.to_str().expect("UTF-8 temp path"));
        assert!(recovered.starts_with(resolver.base_dir()));
        assert_ne!(recovered, absolute);
    }

    #[test]
    fn safe_starting_directories_must_be_inside_the_base() {
        let base = tempfile::tempdir().expect("base dir");
        let resolver = FileResolver::new(base.path(), SafeMode::Safe);
        let inside = resolver.base_dir().join("assets");
        assert_eq!(
            resolver.resolve_directory(inside.to_str().expect("UTF-8 path")),
            inside
        );
        let outside = env::temp_dir().join("outside");
        assert_eq!(
            resolver.resolve_directory(outside.to_str().expect("UTF-8 path")),
            resolver.base_dir()
        );
    }

    #[cfg(unix)]
    #[test]
    fn safe_reads_reject_symlinks_that_leave_the_base() {
        let base = tempfile::tempdir().expect("base dir");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        std::fs::write(outside.path(), "secret").expect("write outside file");

        std::os::unix::fs::symlink(outside.path(), base.path().join("link"))
            .expect("create symlink");
        let resolver = FileResolver::new(base.path(), SafeMode::Safe);
        assert_eq!(resolver.read(None, "link"), Err(FileReadError::Missing));
    }

    #[test]
    fn non_utf8_files_are_reported_as_not_decodable() {
        let base = tempfile::tempdir().expect("base dir");
        std::fs::write(base.path().join("binary"), [0xff, 0xfe]).expect("write binary");
        let resolver = FileResolver::new(base.path(), SafeMode::Unsafe);
        assert_eq!(
            resolver.read(None, "binary"),
            Err(FileReadError::InvalidUtf8)
        );
    }

    #[test]
    fn document_entities_are_decoded_once_for_file_lookup() {
        assert_eq!(decode_document_attribute("img&amp;dir"), "img&dir");
        assert_eq!(
            decode_document_attribute("literal&amp;amp;dir"),
            "literal&amp;dir"
        );
    }

    #[test]
    fn web_paths_preserve_uri_and_protocol_relative_prefixes() {
        assert_eq!(
            resolve_web_path("custom.css", "https://cdn.example/css"),
            "https://cdn.example/css/custom.css"
        );
        assert_eq!(
            resolve_web_path("../custom.css", "//cdn.example/css/themes"),
            "//cdn.example/css/custom.css"
        );
    }

    #[test]
    fn web_paths_normalize_filesystem_style_segments() {
        assert_eq!(
            resolve_web_path("../img/a b.png", "./assets"),
            "./img/a%20b.png"
        );
        assert_eq!(resolve_web_path("/../../img.png", "ignored"), "/img.png");
    }
}
