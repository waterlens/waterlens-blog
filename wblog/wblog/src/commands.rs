use std::collections::BTreeSet;
use std::fs;

use anyhow::{Context as AnyhowContext, Result};

use crate::build_graph::{BuildRequest, BuildSelection, ChangeScope, execute_plan};
use crate::cli::{BuildArgs, BuildFilterArgs, BuildKind};
use crate::output;
use crate::paths::RepoPaths;
use crate::tools::ToolResolver;

#[derive(Clone, Debug)]
pub struct Context {
    pub paths: RepoPaths,
    tools: ToolResolver,
}

impl Context {
    pub fn new(paths: RepoPaths, tools: ToolResolver) -> Self {
        Self { paths, tools }
    }

    pub fn build_incremental(&self, filter: &BuildFilterArgs) -> Result<()> {
        self.build(BuildRequest {
            selection: selection_from_filter(filter),
            full: false,
            changes: None,
        })
    }

    pub fn build_from_args(&self, args: &BuildArgs) -> Result<()> {
        self.build(BuildRequest {
            selection: selection_from_filter(&args.filter),
            full: args.full,
            changes: None,
        })
    }

    pub fn build_for_changes(&self, filter: &BuildFilterArgs, changes: ChangeScope) -> Result<()> {
        self.build(BuildRequest {
            selection: selection_from_filter(filter),
            full: false,
            changes: Some(changes),
        })
    }

    pub fn clean(&self) -> Result<()> {
        for file in self.paths.walk_files(&self.paths.root) {
            if file.file_name().and_then(|name| name.to_str()) == Some(".DS_Store") {
                println!(
                    "{} {}",
                    output::tag_stdout("remove", output::YELLOW),
                    output::accent_stdout(&self.paths.display_path(&file)?, output::DIM),
                );
                fs::remove_file(&file)
                    .with_context(|| format!("failed to remove {}", file.display()))?;
            }
        }

        for dir in [&self.paths.public_dir, &self.paths.cache_dir] {
            if dir.exists() {
                println!(
                    "{} {}",
                    output::tag_stdout("remove", output::YELLOW),
                    output::accent_stdout(&self.paths.display_path(dir)?, output::DIM),
                );
                fs::remove_dir_all(dir)
                    .with_context(|| format!("failed to remove directory {}", dir.display()))?;
            }
        }

        fs::create_dir_all(&self.paths.public_dir).with_context(|| {
            format!(
                "failed to create directory {}",
                self.paths.public_dir.display()
            )
        })?;

        Ok(())
    }

    fn build(&self, request: BuildRequest) -> Result<()> {
        fs::create_dir_all(&self.paths.public_dir).with_context(|| {
            format!(
                "failed to create directory {}",
                self.paths.public_dir.display()
            )
        })?;
        fs::create_dir_all(&self.paths.cache_dir).with_context(|| {
            format!(
                "failed to create directory {}",
                self.paths.cache_dir.display()
            )
        })?;

        execute_plan(&self.paths, &self.tools, request)
    }
}

fn selection_from_filter(filter: &BuildFilterArgs) -> BuildSelection {
    if filter.only.is_empty() {
        BuildSelection::new(BuildKind::ALL)
    } else {
        let selected = filter.only.iter().copied().collect::<BTreeSet<_>>();
        BuildSelection::new(selected)
    }
}
