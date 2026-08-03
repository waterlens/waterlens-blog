//! Parse- and render-time settings for a conversion.

use std::path::{Path, PathBuf};

use asciidoc_parser::{Parser, ReferenceTime, SafeMode, parser::ModificationContext};

use crate::include::FileIncludeHandler;

/// What an attribute directive does to the named attribute.
#[derive(Clone, Debug)]
enum Action {
    /// Assign a value (`-a name=value`).
    Value(String),

    /// Set without a value (`-a name`).
    Set,

    /// Unset (`-a name!`).
    Unset,
}

/// Whether an attribute directive locks out the document or merely seeds a
/// default the document may override.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Precedence {
    /// The caller's value wins and the document cannot change it.
    Override,

    /// The caller supplies a default the document may override.
    Soft,
}

impl Precedence {
    fn modification_context(self) -> ModificationContext {
        match self {
            Self::Override => ModificationContext::ApiOnly,
            Self::Soft => ModificationContext::Anywhere,
        }
    }
}

/// One externally-supplied attribute directive.
#[derive(Clone, Debug)]
struct Directive {
    name: String,
    action: Action,
    precedence: Precedence,
}

/// Settings for a conversion: the externally-supplied document attributes, the
/// safe mode, the input file that anchors `include::` resolution, the
/// reference time, and whether to emit a complete page or body-only output.
///
/// The builder methods take and return `self`, so a configuration reads as one
/// chain:
///
/// ```
/// use asciidoc_waterlens_html5::Options;
///
/// let options = Options::new()
///     .standalone(true)
///     .attribute("lang", "zh-hans");
/// ```
#[derive(Clone, Debug, Default)]
pub struct Options {
    attributes: Vec<Directive>,
    safe_mode: Option<SafeMode>,
    input_file: Option<PathBuf>,
    reference_time: Option<ReferenceTime>,
    standalone: bool,
}

impl Options {
    /// A configuration with nothing set: no attributes, the default safe mode,
    /// and body-only output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns a document attribute, locking it against the document —
    /// Asciidoctor's `-a name=value`.
    #[must_use]
    pub fn attribute<N: Into<String>, V: Into<String>>(mut self, name: N, value: V) -> Self {
        self.attributes.push(Directive {
            name: name.into(),
            action: Action::Value(value.into()),
            precedence: Precedence::Override,
        });
        self
    }

    /// Assigns a document attribute the document may override —
    /// Asciidoctor's `-a name=value@`.
    #[must_use]
    pub fn default_attribute<N: Into<String>, V: Into<String>>(
        mut self,
        name: N,
        value: V,
    ) -> Self {
        self.attributes.push(Directive {
            name: name.into(),
            action: Action::Value(value.into()),
            precedence: Precedence::Soft,
        });
        self
    }

    /// Sets a boolean document attribute, locking it against the document.
    #[must_use]
    pub fn set<N: Into<String>>(mut self, name: N) -> Self {
        self.attributes.push(Directive {
            name: name.into(),
            action: Action::Set,
            precedence: Precedence::Override,
        });
        self
    }

    /// Unsets a document attribute, locking it against the document.
    #[must_use]
    pub fn unset<N: Into<String>>(mut self, name: N) -> Self {
        self.attributes.push(Directive {
            name: name.into(),
            action: Action::Unset,
            precedence: Precedence::Override,
        });
        self
    }

    /// Selects the safe mode.
    ///
    /// The default is [`SafeMode::Unsafe`], matching the `asciidoctor`
    /// command-line tool (the Ruby *API* defaults to `Secure` instead). This
    /// backend exists to build a site from local, trusted sources, where a
    /// document must be able to set `source-highlighter` and read its own
    /// includes.
    #[must_use]
    pub fn safe_mode(mut self, safe_mode: SafeMode) -> Self {
        self.safe_mode = Some(safe_mode);
        self
    }

    /// Records the primary input file, so its `include::` directives resolve
    /// against the file's own directory and the `docfile` attribute family is
    /// populated.
    #[must_use]
    pub fn input_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input_file = Some(path.as_ref().to_path_buf());
        self
    }

    /// Pins the time the date and time attributes (`localyear` among them)
    /// resolve from, so a build is reproducible.
    #[must_use]
    pub fn reference_time(mut self, reference_time: ReferenceTime) -> Self {
        self.reference_time = Some(reference_time);
        self
    }

    /// Selects a complete HTML page (`true`) over body-only output (`false`,
    /// the default).
    #[must_use]
    pub fn standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Whether standalone output was selected.
    pub(crate) fn is_standalone(&self) -> bool {
        self.standalone
    }

    /// The recorded primary input file, if any.
    pub(crate) fn input_file_path(&self) -> Option<&Path> {
        self.input_file.as_deref()
    }

    /// The effective safe mode for parsing and filesystem-backed rendering.
    pub(crate) fn effective_safe_mode(&self) -> SafeMode {
        self.safe_mode.unwrap_or(SafeMode::Unsafe)
    }

    /// Turns `parser` into one configured by these options.
    pub(crate) fn apply(&self, mut parser: Parser) -> Parser {
        // The safe mode is established first: `with_safe_mode` also populates
        // the `safe-mode-*` intrinsic attributes, which a bare `Parser` does
        // not set on its own.
        let safe_mode = self.effective_safe_mode();
        parser = parser.with_safe_mode(safe_mode);

        if let Some(reference_time) = self.reference_time.clone() {
            parser = parser.with_reference_time(reference_time);
        }

        if let Some(input_file) = &self.input_file {
            if let Some(name) = input_file.to_str() {
                parser = parser.with_primary_file_name(name);
            }
            // `include::` directives resolve against the primary file's own
            // directory, the way Asciidoctor resolves them against the
            // including document's directory. Install the handler even when
            // the OS path is not valid UTF-8; only the parser's diagnostic
            // file name requires a `str`.
            parser =
                parser.with_include_file_handler(FileIncludeHandler::new(input_file, safe_mode));
        }

        for directive in &self.attributes {
            let context = directive.precedence.modification_context();
            parser = match &directive.action {
                Action::Value(value) => {
                    parser.with_intrinsic_attribute(&directive.name, value, context)
                }
                Action::Set => parser.with_intrinsic_attribute_bool(&directive.name, true, context),
                Action::Unset => {
                    parser.with_intrinsic_attribute_bool(&directive.name, false, context)
                }
            };
        }

        parser
    }
}
