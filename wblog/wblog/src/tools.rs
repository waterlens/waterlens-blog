/// The external programs the build shells out to.
///
/// AsciiDoc rendering used to live here too, as a path to `asciidoctor`. It is
/// now compiled in (see [`wblog_asciidoc`]), so the build's only remaining
/// external dependencies are Typst and tidy.
#[derive(Clone, Debug)]
pub struct ToolResolver {
    typst: String,
    tidy: String,
}

impl ToolResolver {
    pub fn from_env() -> Self {
        Self {
            typst: resolve("WBLOG_TYPST", "typst"),
            tidy: resolve("WBLOG_TIDY", "tidy"),
        }
    }

    pub fn typst(&self) -> &str {
        &self.typst
    }

    pub fn tidy(&self) -> &str {
        &self.tidy
    }
}

fn resolve(env_key: &str, default: &str) -> String {
    std::env::var(env_key).unwrap_or_else(|_| default.to_owned())
}

#[cfg(test)]
mod tests {
    use super::resolve;

    #[test]
    fn resolve_uses_default_when_env_is_missing() {
        let key = "WBLOG_TEST_TOOL_MISSING";
        let value = resolve(key, "default-tool");
        assert_eq!(value, "default-tool");
    }
}
