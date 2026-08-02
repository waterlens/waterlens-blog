//! AsciiDoc rendering for `wblog`.
//!
//! This crate is the seam between the site builder and the AsciiDoc
//! toolchain. It owns the pairing of [`asciidoc_parser`] with the
//! [`asciidoc_waterlens_html5`] backend, the conversion settings the site
//! builds under, and the reporting of parser warnings — so `wblog` itself
//! never has to know which parser or backend is in play.
//!
//! Rendering used to shell out to `asciidoctor` with a Ruby converter loaded
//! from `tools/asciidoc/`. It now happens in process, which removes the Ruby
//! runtime from the build entirely and makes a render a plain function call.
//!
//! # Examples
//!
//! ```
//! use wblog_asciidoc::Renderer;
//!
//! let renderer = Renderer::new();
//! let rendered = renderer.render_str("= Title\n\nBody.", None);
//! assert!(rendered.html.contains("<title>Title</title>"));
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use asciidoc_waterlens_html5::{Options, ReferenceTime, SafeMode};

/// A warning the parser raised while reading a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    /// The one-based line the warning points at, when the parser located it.
    pub line: Option<usize>,

    /// The human-readable description of the problem.
    pub message: String,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

/// The result of rendering one document.
#[derive(Clone, Debug)]
pub struct Rendered {
    /// The rendered HTML.
    pub html: String,

    /// Warnings the parser raised, in source order. These are advisory: a
    /// document that produces warnings still renders.
    pub warnings: Vec<Warning>,
}

/// Renders AsciiDoc documents to the blog's HTML.
///
/// A renderer carries the settings every document in the site is built under,
/// so the build applies them uniformly. It holds no per-document state and is
/// cheap to clone, so one instance can serve a whole build.
#[derive(Clone, Debug)]
pub struct Renderer {
    safe_mode: SafeMode,
    reference_time: Option<ReferenceTime>,
    attributes: Vec<(String, String)>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    /// A renderer with the site's default settings.
    ///
    /// The safe mode is [`SafeMode::Unsafe`], matching the `asciidoctor`
    /// command line this replaced: the site is built from local, trusted
    /// sources, and documents must be able to select a syntax highlighter and
    /// read their own includes.
    pub fn new() -> Self {
        Self {
            safe_mode: SafeMode::Unsafe,
            reference_time: None,
            attributes: Vec::new(),
        }
    }

    /// Selects a different safe mode.
    #[must_use]
    pub fn with_safe_mode(mut self, safe_mode: SafeMode) -> Self {
        self.safe_mode = safe_mode;
        self
    }

    /// Pins the time that the date and time attributes resolve from.
    ///
    /// The licence footer's closing year comes from the `localyear`
    /// attribute, so pinning the reference time makes a build byte-for-byte
    /// reproducible — which is also what keeps an incremental rebuild from
    /// rewriting every page when the year rolls over mid-build.
    #[must_use]
    pub fn with_reference_time(mut self, reference_time: ReferenceTime) -> Self {
        self.reference_time = Some(reference_time);
        self
    }

    /// Supplies a document attribute to every render, as `-a name=value` did.
    #[must_use]
    pub fn with_attribute<N: Into<String>, V: Into<String>>(mut self, name: N, value: V) -> Self {
        self.attributes.push((name.into(), value.into()));
        self
    }

    /// The conversion options these settings describe.
    fn options(&self, input: Option<&Path>) -> Options {
        let mut options = Options::new().standalone(true).safe_mode(self.safe_mode);

        if let Some(reference_time) = self.reference_time.clone() {
            options = options.reference_time(reference_time);
        }
        if let Some(input) = input {
            options = options.input_file(input);
        }
        for (name, value) in &self.attributes {
            options = options.attribute(name, value);
        }

        options
    }

    /// Renders `source` to a complete HTML page.
    ///
    /// `input` names the file the source came from, when there is one: it
    /// anchors `include::` resolution and populates the `docfile` attribute
    /// family. Pass `None` for source that has no file behind it.
    pub fn render_str(&self, source: &str, input: Option<&Path>) -> Rendered {
        let options = self.options(input);
        let document = asciidoc_waterlens_html5::load_with(source, &options);

        let warnings = document
            .warnings()
            .map(|warning| Warning {
                line: Some(warning.source.line()),
                message: warning.warning.to_string(),
            })
            .collect();

        Rendered {
            html: asciidoc_waterlens_html5::convert_document_with(&document, &options),
            warnings,
        }
    }

    /// Reads the AsciiDoc file at `input` and renders it to a complete HTML
    /// page.
    ///
    /// # Errors
    ///
    /// Returns an error when `input` cannot be read as UTF-8.
    pub fn render_file(&self, input: &Path) -> Result<Rendered> {
        let source = fs::read_to_string(input)
            .with_context(|| format!("failed to read {}", input.display()))?;
        Ok(self.render_str(&source, Some(input)))
    }

    /// Renders the AsciiDoc file at `input` and writes the HTML to `output`,
    /// creating the output's parent directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error when `input` cannot be read, when `output`'s parent
    /// directory cannot be created, or when the HTML cannot be written.
    pub fn render_to_file(&self, input: &Path, output: &Path) -> Result<Vec<Warning>> {
        let rendered = self.render_file(input)?;

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        fs::write(output, &rendered.html)
            .with_context(|| format!("failed to write {}", output.display()))?;

        Ok(rendered.warnings)
    }
}

/// The AsciiDoc files under `root`, in a stable order.
///
/// This is the input discovery the build graph uses: every `.adoc` file below
/// the content directory, sorted so the build visits them deterministically.
pub fn adoc_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_adoc_files(root, &mut files);
    files.sort();
    files
}

/// Walks `dir` depth-first, pushing every `.adoc` file onto `files`.
fn collect_adoc_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_adoc_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "adoc") {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_standalone_page() {
        let rendered = Renderer::new().render_str("= Title\n\nBody.", None);
        assert!(rendered.html.starts_with("<!DOCTYPE html>"));
        assert!(rendered.html.contains("<title>Title</title>"));
        assert!(rendered.html.contains("<p>Body.</p>"));
        // Like Asciidoctor's, the converted output carries no trailing
        // newline.
        assert!(rendered.html.ends_with("</html>"));
    }

    #[test]
    fn renders_a_file_and_writes_it_to_a_temp_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let input = dir.path().join("post.adoc");
        let output = dir.path().join("nested/post.html");
        fs::write(&input, "= Post\n:lang: en\n\nHello.").expect("write input");

        let warnings = Renderer::new()
            .render_to_file(&input, &output)
            .expect("render");
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");

        let html = fs::read_to_string(&output).expect("read output");
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<h1>Post</h1>"));
    }

    #[test]
    fn a_pinned_reference_time_fixes_the_footer_year() {
        let renderer = Renderer::new().with_reference_time(ReferenceTime::from_unix_timestamp(0));
        let rendered = renderer.render_str("= Title\n\nBody.", None);
        assert!(rendered.html.contains("2021 - 1970"));
    }

    #[test]
    fn adoc_files_finds_documents_recursively_in_a_stable_order() {
        let dir = tempfile::tempdir().expect("create temp dir");
        fs::create_dir_all(dir.path().join("zh/posts")).expect("create dirs");
        fs::write(dir.path().join("index.adoc"), "= Index").expect("write");
        fs::write(dir.path().join("zh/posts/a.adoc"), "= A").expect("write");
        fs::write(dir.path().join("zh/posts/ignore.txt"), "no").expect("write");

        let files = adoc_files(dir.path());
        let names: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(dir.path())
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        assert_eq!(names, ["index.adoc", "zh/posts/a.adoc"]);
    }
}
