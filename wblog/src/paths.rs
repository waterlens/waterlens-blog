use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct RepoPaths {
    pub root: PathBuf,
    pub public_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub content_dir: PathBuf,
    pub static_dir: PathBuf,
    pub resource_typst_dir: PathBuf,
    pub resource_svg_dir: PathBuf,
    pub sass_style: PathBuf,
    pub tidy_config: PathBuf,
    pub w_asciidoc_dir: PathBuf,
}

impl RepoPaths {
    pub fn from_current_dir() -> Result<Self> {
        let root = std::env::current_dir().context("failed to read current directory")?;

        Ok(Self {
            public_dir: root.join("public"),
            cache_dir: root.join(".wblog-cache"),
            content_dir: root.join("content"),
            static_dir: root.join("static"),
            resource_typst_dir: root.join("resource/typst"),
            resource_svg_dir: root.join("resource/svg"),
            sass_style: root.join("styles/style.scss"),
            tidy_config: root.join("tidy.cfg"),
            w_asciidoc_dir: root.join("tools/asciidoc"),
            root,
        })
    }

    pub fn public_resource_dir(&self) -> PathBuf {
        self.public_dir.join("resource")
    }

    pub fn sass_dir(&self) -> &Path {
        self.sass_style
            .parent()
            .expect("styles/style.scss must have a parent")
    }

    pub fn cache_db_path(&self) -> PathBuf {
        self.cache_dir.join("n2o5-dumb-db.bin")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.cache_dir.join("managed-outputs.tsv")
    }

    pub fn adoc_stage_output(&self, input: &Path) -> Result<PathBuf> {
        let relative = input.strip_prefix(&self.content_dir).map_err(|_| {
            anyhow!(
                "{} is not under {}",
                input.display(),
                self.content_dir.display()
            )
        })?;
        Ok(self
            .cache_dir
            .join("stage/adoc")
            .join(relative)
            .with_extension("html"))
    }

    pub fn static_html_stage_output(&self, input: &Path) -> Result<PathBuf> {
        let relative = input.strip_prefix(&self.static_dir).map_err(|_| {
            anyhow!(
                "{} is not under {}",
                input.display(),
                self.static_dir.display()
            )
        })?;
        Ok(self.cache_dir.join("stage/static-html").join(relative))
    }

    pub fn relative<'a>(&self, path: &'a Path) -> Result<&'a Path> {
        path.strip_prefix(&self.root)
            .map_err(|_| anyhow!("path {} is outside repo root", path.display()))
    }

    pub fn resolve_rooted_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            normalize_path(path)
        } else {
            normalize_path(&self.root.join(path))
        }
    }

    pub fn is_under_root(&self, path: &Path) -> bool {
        normalize_path(path).starts_with(normalize_path(&self.root))
    }

    pub fn display_path(&self, path: &Path) -> Result<String> {
        Ok(self.relative(path)?.to_string_lossy().into_owned())
    }

    pub fn typst_output(&self, input: &Path) -> Result<PathBuf> {
        let relative = input.strip_prefix(&self.resource_typst_dir).map_err(|_| {
            anyhow!(
                "{} is not under {}",
                input.display(),
                self.resource_typst_dir.display()
            )
        })?;
        Ok(self
            .public_resource_dir()
            .join(relative)
            .with_extension("svg"))
    }

    pub fn svg_output(&self, input: &Path) -> Result<PathBuf> {
        let relative = input.strip_prefix(&self.resource_svg_dir).map_err(|_| {
            anyhow!(
                "{} is not under {}",
                input.display(),
                self.resource_svg_dir.display()
            )
        })?;
        Ok(self.public_resource_dir().join(relative))
    }

    pub fn adoc_output(&self, input: &Path) -> Result<PathBuf> {
        let relative = input.strip_prefix(&self.content_dir).map_err(|_| {
            anyhow!(
                "{} is not under {}",
                input.display(),
                self.content_dir.display()
            )
        })?;
        Ok(self.public_dir.join(relative).with_extension("html"))
    }

    pub fn asset_output(&self, input: &Path) -> Result<PathBuf> {
        let relative = input.strip_prefix(&self.static_dir).map_err(|_| {
            anyhow!(
                "{} is not under {}",
                input.display(),
                self.static_dir.display()
            )
        })?;
        Ok(self.public_dir.join(relative))
    }

    pub fn iter_extension(&self, dir: &Path, extension: &str) -> Vec<PathBuf> {
        self.walk_files(dir)
            .into_iter()
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(extension))
            .collect()
    }

    pub fn walk_files(&self, dir: &Path) -> Vec<PathBuf> {
        if !dir.exists() {
            return Vec::new();
        }

        let mut files = WalkDir::new(dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .collect::<Vec<_>>();
        files.sort();
        files
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::RepoPaths;
    use tempfile::TempDir;

    struct Fixture {
        _tempdir: TempDir,
        paths: RepoPaths,
    }

    fn fixture() -> Fixture {
        let tempdir = tempfile::tempdir().unwrap();
        let root = tempdir.path().to_path_buf();
        Fixture {
            _tempdir: tempdir,
            paths: RepoPaths {
                root: root.clone(),
                public_dir: root.join("public"),
                cache_dir: root.join(".wblog-cache"),
                content_dir: root.join("content"),
                static_dir: root.join("static"),
                resource_typst_dir: root.join("resource/typst"),
                resource_svg_dir: root.join("resource/svg"),
                sass_style: root.join("styles/style.scss"),
                tidy_config: root.join("tidy.cfg"),
                w_asciidoc_dir: root.join("tools/asciidoc"),
            },
        }
    }

    #[test]
    fn typst_output_keeps_relative_structure() {
        let fixture = fixture();
        let input = fixture.paths.root.join("resource/typst/foo/bar.typ");
        let output = fixture.paths.typst_output(&input).unwrap();
        assert_eq!(
            output,
            fixture.paths.root.join("public/resource/foo/bar.svg")
        );
    }

    #[test]
    fn adoc_stage_output_uses_cache_directory() {
        let fixture = fixture();
        let input = fixture.paths.root.join("content/zh/posts/demo.adoc");
        let output = fixture.paths.adoc_stage_output(&input).unwrap();
        assert_eq!(
            output,
            fixture
                .paths
                .root
                .join(".wblog-cache/stage/adoc/zh/posts/demo.html")
        );
    }

    #[test]
    fn static_html_stage_output_uses_cache_directory() {
        let fixture = fixture();
        let input = fixture.paths.root.join("static/demo/index.html");
        let output = fixture.paths.static_html_stage_output(&input).unwrap();
        assert_eq!(
            output,
            fixture
                .paths
                .root
                .join(".wblog-cache/stage/static-html/demo/index.html")
        );
    }

    #[test]
    fn resolve_rooted_path_normalizes_parent_segments() {
        let fixture = fixture();
        let output = fixture
            .paths
            .resolve_rooted_path(Path::new("public/preview/../site"));
        assert_eq!(output, fixture.paths.root.join("public/site"));
    }

    #[test]
    fn is_under_root_rejects_parent_escape() {
        let fixture = fixture();
        let outside = fixture.paths.root.join("../preview");
        assert!(!fixture.paths.is_under_root(&outside));
    }
}
