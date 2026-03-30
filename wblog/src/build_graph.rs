use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use grass::{Options as GrassOptions, OutputStyle};
use n2o5::db::dumb::DumbDb;
use n2o5::graph::{BuildId, BuildMethod, BuildNode, GraphBuilder, hash_build};
use n2o5::{ExecConfig, ExecDb, Executor};

use crate::cli::BuildKind;
use crate::console_progress::ConsoleProgress;
use crate::output;
use crate::paths::RepoPaths;
use crate::tools::ToolResolver;

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, Debug)]
pub struct BuildSelection {
    kinds: BTreeSet<BuildKind>,
}

impl BuildSelection {
    pub fn new(kinds: impl IntoIterator<Item = BuildKind>) -> Self {
        Self {
            kinds: kinds.into_iter().collect(),
        }
    }

    pub fn contains(&self, kind: BuildKind) -> bool {
        self.kinds.contains(&kind)
    }
}

#[derive(Clone, Debug)]
pub struct BuildRequest {
    pub selection: BuildSelection,
    pub full: bool,
    pub changes: Option<ChangeScope>,
}

#[derive(Clone, Debug, Default)]
pub struct ChangeScope {
    sources: BTreeSet<PathBuf>,
    sass_tree_changed: bool,
    adoc_tooling_changed: bool,
    tidy_config_changed: bool,
}

impl ChangeScope {
    pub fn from_events(paths: &RepoPaths, event_paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut scope = Self::default();

        for path in event_paths {
            if path.starts_with(&paths.content_dir)
                || path.starts_with(&paths.resource_typst_dir)
                || path.starts_with(&paths.resource_svg_dir)
                || path.starts_with(&paths.static_dir)
            {
                scope.sources.insert(path);
                continue;
            }

            if path.starts_with(&paths.w_asciidoc_dir) {
                scope.adoc_tooling_changed = true;
                continue;
            }

            if path.starts_with(paths.sass_dir()) {
                scope.sass_tree_changed = true;
                continue;
            }

            if path == paths.tidy_config {
                scope.tidy_config_changed = true;
            }
        }

        scope
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
            && !self.sass_tree_changed
            && !self.adoc_tooling_changed
            && !self.tidy_config_changed
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TargetFamily {
    Typst,
    Css,
    Svg,
    Static,
    AdocHtml,
    StaticHtml,
}

#[derive(Clone, Debug)]
struct BuildTarget {
    id: BuildId,
    kind: BuildKind,
    family: TargetFamily,
    source: PathBuf,
    output: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ManagedGroup {
    Css,
    Typst,
    Svg,
    AdocStage,
    AdocHtml,
    StaticHtmlStage,
    StaticHtml,
    Static,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ManagedOutput {
    group: ManagedGroup,
    path: PathBuf,
}

struct PlannedGraph {
    graph: n2o5::BuildGraph,
    targets: Vec<BuildTarget>,
    managed_outputs: BTreeSet<ManagedOutput>,
}

pub fn execute_plan(paths: &RepoPaths, tools: &ToolResolver, request: BuildRequest) -> Result<()> {
    let planned = build_graph(paths, tools)?;
    cleanup_stale_outputs(paths, &planned, &request.selection)?;
    let wanted = planned.select(
        paths,
        &request.selection,
        request.full,
        request.changes.as_ref(),
    );
    if wanted.is_empty() {
        write_manifest(paths, &planned.managed_outputs)?;
        println!(
            "{} {}",
            output::tag_stdout("skip", output::BLUE),
            output::accent_stdout("No matching targets.", output::DIM),
        );
        return Ok(());
    }

    let db = DumbDb::new(&paths.cache_db_path())
        .with_context(|| format!("failed to open {}", paths.cache_db_path().display()))?;
    if request.full {
        invalidate_targets(&db, &planned.graph, &wanted)?;
    }

    let progress = ConsoleProgress;
    let cfg = ExecConfig {
        parallelism: std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1),
    };

    let mut executor = Executor::new(&cfg, &planned.graph, &db, &progress, &());
    executor.want(wanted);
    executor.run().map_err(|error| anyhow!(error))?;
    write_manifest(paths, &planned.managed_outputs)?;
    Ok(())
}

impl PlannedGraph {
    fn select(
        &self,
        paths: &RepoPaths,
        selection: &BuildSelection,
        full: bool,
        changes: Option<&ChangeScope>,
    ) -> Vec<BuildId> {
        let mut wanted = BTreeSet::new();

        for target in &self.targets {
            if !selection.contains(target.kind) {
                continue;
            }

            if let Some(scope) = changes {
                if !matches_target(scope, target) {
                    continue;
                }
            } else if !full && !is_dirty(paths, target) {
                continue;
            }

            wanted.insert(target.id);
        }

        wanted.into_iter().collect()
    }
}

fn is_dirty(paths: &RepoPaths, target: &BuildTarget) -> bool {
    if !target.output.exists() {
        return true;
    }

    let output_mtime = modified(&target.output);

    match target.family {
        TargetFamily::Typst | TargetFamily::Svg | TargetFamily::Static => {
            modified(&target.source) > output_mtime
        }
        TargetFamily::Css => paths
            .walk_files(paths.sass_dir())
            .into_iter()
            .any(|path| modified(&path) > output_mtime),
        TargetFamily::AdocHtml => {
            modified(&target.source) > output_mtime
                || modified(&paths.tidy_config) > output_mtime
                || paths
                    .walk_files(&paths.w_asciidoc_dir)
                    .into_iter()
                    .any(|path| modified(&path) > output_mtime)
        }
        TargetFamily::StaticHtml => {
            modified(&target.source) > output_mtime || modified(&paths.tidy_config) > output_mtime
        }
    }
}

fn modified(path: &Path) -> std::time::SystemTime {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

fn matches_target(scope: &ChangeScope, target: &BuildTarget) -> bool {
    if scope.is_empty() {
        return true;
    }

    if scope.sources.contains(&target.source) {
        return true;
    }

    match target.family {
        TargetFamily::Css => scope.sass_tree_changed,
        TargetFamily::AdocHtml => scope.adoc_tooling_changed || scope.tidy_config_changed,
        TargetFamily::StaticHtml => scope.tidy_config_changed,
        TargetFamily::Typst | TargetFamily::Svg | TargetFamily::Static => false,
    }
}

fn build_graph(paths: &RepoPaths, tools: &ToolResolver) -> Result<PlannedGraph> {
    let mut builder = GraphBuilder::new();
    let mut targets = Vec::new();
    let mut managed_outputs = BTreeSet::new();

    let tidy_cfg_id = paths
        .tidy_config
        .exists()
        .then(|| builder.add_file(&paths.tidy_config));

    if paths.sass_style.exists() {
        let css_input_id = builder.add_file(&paths.sass_style);
        let css_output = paths.public_dir.join("style.css");
        let css_output_id = builder.add_file(&css_output);
        let css_node = builder.add_build(BuildNode {
            command: grass_callback(
                format!("Render {}", paths.display_path(&paths.sass_style)?),
                paths.sass_style.clone(),
                paths.sass_dir().to_path_buf(),
                css_output.clone(),
            ),
            ins: vec![css_input_id],
            outs: vec![css_output_id],
            description: Some(format!("Render {}", paths.display_path(&paths.sass_style)?).into()),
        });
        targets.push(BuildTarget {
            id: css_node,
            kind: BuildKind::Css,
            family: TargetFamily::Css,
            source: paths.sass_style.clone(),
            output: css_output.clone(),
        });
        managed_outputs.insert(ManagedOutput {
            group: ManagedGroup::Css,
            path: css_output,
        });
    }

    for input in paths.iter_extension(&paths.resource_typst_dir, "typ") {
        let output = paths.typst_output(&input)?;
        let input_id = builder.add_file(&input);
        let output_id = builder.add_file(&output);
        let description = format!(
            "Typst {} -> {}",
            paths.display_path(&input)?,
            paths.display_path(&output)?
        );
        let node = builder.add_build(BuildNode {
            command: command_callback(
                description.clone(),
                paths.root.clone(),
                tools.typst().to_owned(),
                vec![
                    "compile".to_owned(),
                    "-f".to_owned(),
                    "svg".to_owned(),
                    paths.display_path(&input)?,
                    paths.display_path(&output)?,
                ],
                vec![output.clone()],
            ),
            ins: vec![input_id],
            outs: vec![output_id],
            description: Some(description.clone().into()),
        });
        targets.push(BuildTarget {
            id: node,
            kind: BuildKind::Typst,
            family: TargetFamily::Typst,
            source: input,
            output: output.clone(),
        });
        managed_outputs.insert(ManagedOutput {
            group: ManagedGroup::Typst,
            path: output,
        });
    }

    for input in paths.walk_files(&paths.resource_svg_dir) {
        let output = paths.svg_output(&input)?;
        let input_id = builder.add_file(&input);
        let output_id = builder.add_file(&output);
        let description = format!(
            "Copy {} -> {}",
            paths.display_path(&input)?,
            paths.display_path(&output)?
        );
        let node = builder.add_build(BuildNode {
            command: copy_callback(description.clone(), input.clone(), output.clone()),
            ins: vec![input_id],
            outs: vec![output_id],
            description: Some(description.clone().into()),
        });
        targets.push(BuildTarget {
            id: node,
            kind: BuildKind::Svg,
            family: TargetFamily::Svg,
            source: input,
            output: output.clone(),
        });
        managed_outputs.insert(ManagedOutput {
            group: ManagedGroup::Svg,
            path: output,
        });
    }

    let adoc_inputs = paths.iter_extension(&paths.content_dir, "adoc");
    if !adoc_inputs.is_empty() {
        let tidy_cfg_id = tidy_cfg_id
            .ok_or_else(|| anyhow!("expected file {} to exist", paths.tidy_config.display()))?;
        let adoc_support_files = adoc_support_files(paths)?;
        let adoc_support_inputs = adoc_support_files
            .iter()
            .map(|path| builder.add_file(path))
            .collect::<Vec<_>>();

        for input in adoc_inputs {
            let stage_output = paths.adoc_stage_output(&input)?;
            let final_output = paths.adoc_output(&input)?;
            let input_id = builder.add_file(&input);
            let stage_output_id = builder.add_file(&stage_output);
            let final_output_id = builder.add_file(&final_output);

            let mut ins = vec![input_id];
            ins.extend(adoc_support_inputs.iter().copied());

            let render_description = format!(
                "Render {} -> {}",
                paths.display_path(&input)?,
                paths.display_path(&stage_output)?
            );
            let render_node = builder.add_build(BuildNode {
                command: command_callback(
                    render_description.clone(),
                    paths.root.clone(),
                    tools.asciidoctor().to_owned(),
                    vec![
                        "-b".to_owned(),
                        "w-html".to_owned(),
                        "-r".to_owned(),
                        "./tools/asciidoc/convert.rb".to_owned(),
                        "-r".to_owned(),
                        "./tools/asciidoc/hljs.rb".to_owned(),
                        paths.display_path(&input)?,
                        "-o".to_owned(),
                        paths.display_path(&stage_output)?,
                    ],
                    vec![stage_output.clone()],
                ),
                ins,
                outs: vec![stage_output_id],
                description: Some(render_description.into()),
            });

            let publish_description = format!(
                "Publish {} -> {}",
                paths.display_path(&stage_output)?,
                paths.display_path(&final_output)?
            );
            let publish_node = builder.add_build(BuildNode {
                command: publish_html_callback(
                    publish_description.clone(),
                    paths.root.clone(),
                    tools.tidy().to_owned(),
                    stage_output.clone(),
                    final_output.clone(),
                    paths.tidy_config.clone(),
                ),
                ins: vec![stage_output_id, tidy_cfg_id],
                outs: vec![final_output_id],
                description: Some(publish_description.into()),
            });
            builder.add_build_dep(publish_node, render_node);

            targets.push(BuildTarget {
                id: publish_node,
                kind: BuildKind::Adoc,
                family: TargetFamily::AdocHtml,
                source: input.clone(),
                output: final_output.clone(),
            });
            targets.push(BuildTarget {
                id: publish_node,
                kind: BuildKind::Tidy,
                family: TargetFamily::AdocHtml,
                source: input,
                output: final_output.clone(),
            });
            managed_outputs.insert(ManagedOutput {
                group: ManagedGroup::AdocStage,
                path: stage_output,
            });
            managed_outputs.insert(ManagedOutput {
                group: ManagedGroup::AdocHtml,
                path: final_output,
            });
        }
    }

    let static_inputs = paths.walk_files(&paths.static_dir);
    let has_static_html = static_inputs
        .iter()
        .any(|path| path.extension().and_then(|ext| ext.to_str()) == Some("html"));
    if has_static_html && tidy_cfg_id.is_none() {
        bail!("expected file {} to exist", paths.tidy_config.display());
    }

    for input in static_inputs {
        if input.extension().and_then(|ext| ext.to_str()) == Some("html") {
            let tidy_cfg_id = tidy_cfg_id
                .ok_or_else(|| anyhow!("expected file {} to exist", paths.tidy_config.display()))?;
            let stage_output = paths.static_html_stage_output(&input)?;
            let final_output = paths.asset_output(&input)?;
            let input_id = builder.add_file(&input);
            let stage_output_id = builder.add_file(&stage_output);
            let final_output_id = builder.add_file(&final_output);

            let stage_description = format!(
                "Stage {} -> {}",
                paths.display_path(&input)?,
                paths.display_path(&stage_output)?
            );
            let stage_node = builder.add_build(BuildNode {
                command: copy_callback(
                    stage_description.clone(),
                    input.clone(),
                    stage_output.clone(),
                ),
                ins: vec![input_id],
                outs: vec![stage_output_id],
                description: Some(stage_description.into()),
            });

            let publish_description = format!(
                "Publish {} -> {}",
                paths.display_path(&stage_output)?,
                paths.display_path(&final_output)?
            );
            let publish_node = builder.add_build(BuildNode {
                command: publish_html_callback(
                    publish_description.clone(),
                    paths.root.clone(),
                    tools.tidy().to_owned(),
                    stage_output.clone(),
                    final_output.clone(),
                    paths.tidy_config.clone(),
                ),
                ins: vec![stage_output_id, tidy_cfg_id],
                outs: vec![final_output_id],
                description: Some(publish_description.into()),
            });
            builder.add_build_dep(publish_node, stage_node);

            targets.push(BuildTarget {
                id: publish_node,
                kind: BuildKind::Static,
                family: TargetFamily::StaticHtml,
                source: input.clone(),
                output: final_output.clone(),
            });
            targets.push(BuildTarget {
                id: publish_node,
                kind: BuildKind::Tidy,
                family: TargetFamily::StaticHtml,
                source: input,
                output: final_output.clone(),
            });
            managed_outputs.insert(ManagedOutput {
                group: ManagedGroup::StaticHtmlStage,
                path: stage_output,
            });
            managed_outputs.insert(ManagedOutput {
                group: ManagedGroup::StaticHtml,
                path: final_output,
            });
            continue;
        }

        let output = paths.asset_output(&input)?;
        let input_id = builder.add_file(&input);
        let output_id = builder.add_file(&output);
        let description = format!(
            "Copy {} -> {}",
            paths.display_path(&input)?,
            paths.display_path(&output)?
        );
        let node = builder.add_build(BuildNode {
            command: copy_callback(description.clone(), input.clone(), output.clone()),
            ins: vec![input_id],
            outs: vec![output_id],
            description: Some(description.into()),
        });
        targets.push(BuildTarget {
            id: node,
            kind: BuildKind::Static,
            family: TargetFamily::Static,
            source: input,
            output: output.clone(),
        });
        managed_outputs.insert(ManagedOutput {
            group: ManagedGroup::Static,
            path: output,
        });
    }

    Ok(PlannedGraph {
        graph: builder.build().map_err(|error| anyhow!(error))?,
        targets,
        managed_outputs,
    })
}

fn invalidate_targets(db: &DumbDb, graph: &n2o5::BuildGraph, targets: &[BuildId]) -> Result<()> {
    let mut txn = db.begin_write();
    for id in targets {
        let node = graph
            .lookup_build(*id)
            .ok_or_else(|| anyhow!("unknown build id {id:?}"))?;
        let hash = hash_build(node, graph);
        txn.invalidate_build(hash);
        for file_id in &node.outs {
            let path = graph
                .lookup_path(*file_id)
                .ok_or_else(|| anyhow!("unknown file id {file_id:?}"))?;
            txn.invalidate_file(path);
        }
    }
    txn.commit();
    Ok(())
}

fn cleanup_stale_outputs(
    paths: &RepoPaths,
    planned: &PlannedGraph,
    selection: &BuildSelection,
) -> Result<()> {
    let previous = read_manifest(paths)?;
    let current_paths = planned
        .managed_outputs
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    let selected_groups = selected_groups(selection);

    for entry in previous {
        if !selected_groups.contains(&entry.group) {
            continue;
        }
        if planned.managed_outputs.contains(&entry) {
            continue;
        }
        if current_paths.contains(&entry.path) {
            continue;
        }
        remove_managed_output(&entry.path)?;
    }

    Ok(())
}

fn selected_groups(selection: &BuildSelection) -> BTreeSet<ManagedGroup> {
    let mut groups = BTreeSet::new();

    for kind in &selection.kinds {
        match kind {
            BuildKind::Typst => {
                groups.insert(ManagedGroup::Typst);
            }
            BuildKind::Adoc => {
                groups.insert(ManagedGroup::AdocStage);
                groups.insert(ManagedGroup::AdocHtml);
            }
            BuildKind::Css => {
                groups.insert(ManagedGroup::Css);
            }
            BuildKind::Svg => {
                groups.insert(ManagedGroup::Svg);
            }
            BuildKind::Static => {
                groups.insert(ManagedGroup::Static);
                groups.insert(ManagedGroup::StaticHtmlStage);
                groups.insert(ManagedGroup::StaticHtml);
            }
            BuildKind::Tidy => {
                groups.insert(ManagedGroup::AdocStage);
                groups.insert(ManagedGroup::AdocHtml);
                groups.insert(ManagedGroup::StaticHtmlStage);
                groups.insert(ManagedGroup::StaticHtml);
            }
        }
    }

    groups
}

fn adoc_support_files(paths: &RepoPaths) -> Result<Vec<PathBuf>> {
    let files = paths.walk_files(&paths.w_asciidoc_dir);
    if files.is_empty() {
        bail!(
            "expected AsciiDoc support files under {}",
            paths.w_asciidoc_dir.display()
        );
    }

    for file in [
        paths.w_asciidoc_dir.join("convert.rb"),
        paths.w_asciidoc_dir.join("hljs.rb"),
    ] {
        if !file.exists() {
            bail!("expected file {} to exist", file.display());
        }
    }

    Ok(files)
}

fn read_manifest(paths: &RepoPaths) -> Result<BTreeSet<ManagedOutput>> {
    let manifest_path = paths.manifest_path();
    if !manifest_path.exists() {
        return Ok(BTreeSet::new());
    }

    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let mut outputs = BTreeSet::new();
    for (index, line) in content.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let (group, relative_path) = line.split_once('\t').ok_or_else(|| {
            anyhow!(
                "invalid manifest line {} in {}",
                index + 1,
                manifest_path.display()
            )
        })?;
        let path = paths.resolve_rooted_path(Path::new(relative_path));
        if !paths.is_under_root(&path) {
            continue;
        }
        outputs.insert(ManagedOutput {
            group: parse_group(group)?,
            path,
        });
    }
    Ok(outputs)
}

fn write_manifest(paths: &RepoPaths, managed_outputs: &BTreeSet<ManagedOutput>) -> Result<()> {
    let manifest_path = paths.manifest_path();
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut body = String::new();
    for entry in managed_outputs {
        if !paths.is_under_root(&entry.path) {
            bail!(
                "managed output {} must stay under repo root {}",
                entry.path.display(),
                paths.root.display()
            );
        }
        let relative = paths.relative(&entry.path).with_context(|| {
            format!(
                "managed output {} must stay under repo root {}",
                entry.path.display(),
                paths.root.display()
            )
        })?;
        body.push_str(group_name(entry.group));
        body.push('\t');
        body.push_str(&relative.to_string_lossy());
        body.push('\n');
    }
    fs::write(&manifest_path, body)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    Ok(())
}

fn parse_group(name: &str) -> Result<ManagedGroup> {
    match name {
        "css" => Ok(ManagedGroup::Css),
        "typst" => Ok(ManagedGroup::Typst),
        "svg" => Ok(ManagedGroup::Svg),
        "adoc-stage" => Ok(ManagedGroup::AdocStage),
        "adoc-html" => Ok(ManagedGroup::AdocHtml),
        "static-html-stage" => Ok(ManagedGroup::StaticHtmlStage),
        "static-html" => Ok(ManagedGroup::StaticHtml),
        "assets" | "static" => Ok(ManagedGroup::Static),
        _ => bail!("unknown manifest group {name}"),
    }
}

fn group_name(group: ManagedGroup) -> &'static str {
    match group {
        ManagedGroup::Css => "css",
        ManagedGroup::Typst => "typst",
        ManagedGroup::Svg => "svg",
        ManagedGroup::AdocStage => "adoc-stage",
        ManagedGroup::AdocHtml => "adoc-html",
        ManagedGroup::StaticHtmlStage => "static-html-stage",
        ManagedGroup::StaticHtml => "static-html",
        ManagedGroup::Static => "static",
    }
}

fn remove_managed_output(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path)
        .with_context(|| format!("failed to remove stale output {}", path.display()))?;
    remove_empty_parents(path.parent());
    Ok(())
}

fn remove_empty_parents(mut current: Option<&Path>) {
    while let Some(path) = current {
        match fs::remove_dir(path) {
            Ok(()) => current = path.parent(),
            Err(_) => break,
        }
    }
}

fn command_callback(
    _name: String,
    cwd: PathBuf,
    program: String,
    args: Vec<String>,
    outputs: Vec<PathBuf>,
) -> BuildMethod {
    let signature = format!("{program} {}", args.join(" "));
    BuildMethod::Callback(
        signature.into(),
        Box::new(move |_| {
            ensure_outputs(&outputs)?;
            run_process(&cwd, &program, &args).map_err(io_to_dyn)?;
            Ok(())
        }),
    )
}

fn copy_callback(name: String, input: PathBuf, output: PathBuf) -> BuildMethod {
    BuildMethod::Callback(
        name.into(),
        Box::new(move |_| {
            ensure_outputs(std::slice::from_ref(&output))?;
            fs::copy(&input, &output).map_err(io_to_dyn)?;
            Ok(())
        }),
    )
}

fn grass_callback(
    name: String,
    input: PathBuf,
    load_path: PathBuf,
    output: PathBuf,
) -> BuildMethod {
    BuildMethod::Callback(
        name.into(),
        Box::new(move |_| {
            ensure_outputs(std::slice::from_ref(&output))?;
            let css = grass::from_path(
                &input,
                &GrassOptions::default()
                    .load_path(&load_path)
                    .style(OutputStyle::Expanded),
            )
            .map_err(|error| -> DynError { Box::new(error) })?;
            fs::write(&output, css).map_err(io_to_dyn)?;
            Ok(())
        }),
    )
}

fn publish_html_callback(
    name: String,
    cwd: PathBuf,
    tidy_program: String,
    stage_input: PathBuf,
    final_output: PathBuf,
    tidy_config: PathBuf,
) -> BuildMethod {
    BuildMethod::Callback(
        name.into(),
        Box::new(move |_| {
            ensure_outputs(std::slice::from_ref(&final_output))?;
            fs::copy(&stage_input, &final_output).map_err(io_to_dyn)?;
            let args = vec![
                "-config".to_owned(),
                display_relative(&cwd, &tidy_config),
                display_relative(&cwd, &final_output),
            ];
            run_process(&cwd, &tidy_program, &args).map_err(io_to_dyn)?;
            Ok(())
        }),
    )
}

fn ensure_outputs(outputs: &[PathBuf]) -> Result<(), DynError> {
    for output in outputs {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(io_to_dyn)?;
        } else {
            return Err(string_to_dyn(format!(
                "path {} has no parent directory",
                output.display()
            )));
        }
    }
    Ok(())
}

fn run_process(cwd: &Path, program: &str, args: &[String]) -> io::Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{program} exited with status {status}"
        )))
    }
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn io_to_dyn(error: io::Error) -> DynError {
    Box::new(error)
}

fn string_to_dyn(message: String) -> DynError {
    Box::new(io::Error::other(message))
}

#[cfg(test)]
mod tests {
    use super::{
        BuildSelection, ChangeScope, ManagedGroup, ManagedOutput, PlannedGraph, TargetFamily,
        cleanup_stale_outputs, matches_target, read_manifest, write_manifest,
    };
    use crate::cli::BuildKind;
    use crate::paths::RepoPaths;
    use n2o5::graph::{BuildMethod, BuildNode, GraphBuilder};
    use tempfile::TempDir;

    fn dummy_build_id() -> n2o5::BuildId {
        let mut builder = GraphBuilder::new();
        let id = builder.add_build(BuildNode {
            command: BuildMethod::Phony,
            ins: vec![],
            outs: vec![],
            description: None,
        });
        id
    }

    fn temp_root() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn repo_paths(root: &std::path::Path) -> RepoPaths {
        RepoPaths {
            root: root.to_path_buf(),
            public_dir: root.join("public"),
            cache_dir: root.join(".wblog-cache"),
            content_dir: root.join("content"),
            static_dir: root.join("static"),
            resource_typst_dir: root.join("resource/typst"),
            resource_svg_dir: root.join("resource/svg"),
            sass_style: root.join("styles/style.scss"),
            tidy_config: root.join("tidy.cfg"),
            w_asciidoc_dir: root.join("tools/asciidoc"),
        }
    }

    fn empty_planned(managed_outputs: impl IntoIterator<Item = ManagedOutput>) -> PlannedGraph {
        let builder = GraphBuilder::new();
        PlannedGraph {
            graph: builder.build().unwrap(),
            targets: Vec::new(),
            managed_outputs: managed_outputs.into_iter().collect(),
        }
    }

    #[test]
    fn selection_contains_requested_kinds() {
        let selection = BuildSelection::new([BuildKind::Typst, BuildKind::Adoc]);
        assert!(selection.contains(BuildKind::Typst));
        assert!(!selection.contains(BuildKind::Css));
    }

    #[test]
    fn content_change_matches_adoc_family() {
        let root = temp_root();
        let repo = root.path();
        let scope = ChangeScope {
            sources: [repo.join("content/post.adoc")].into_iter().collect(),
            ..ChangeScope::default()
        };
        let target = super::BuildTarget {
            id: dummy_build_id(),
            kind: BuildKind::Adoc,
            family: TargetFamily::AdocHtml,
            source: repo.join("content/post.adoc"),
            output: repo.join("public/post.html"),
        };
        assert!(matches_target(&scope, &target));
    }

    #[test]
    fn tidy_config_change_matches_static_html_publishers() {
        let root = temp_root();
        let repo = root.path();
        let scope = ChangeScope {
            tidy_config_changed: true,
            ..ChangeScope::default()
        };
        let target = super::BuildTarget {
            id: dummy_build_id(),
            kind: BuildKind::Static,
            family: TargetFamily::StaticHtml,
            source: repo.join("static/index.html"),
            output: repo.join("public/index.html"),
        };
        assert!(matches_target(&scope, &target));
    }

    #[test]
    fn cleanup_stale_outputs_removes_deleted_adoc_outputs() {
        let root = temp_root();
        let repo = root.path();
        let paths = repo_paths(repo);
        let final_output = paths.public_dir.join("posts/old.html");
        let stage_output = paths.cache_dir.join("stage/adoc/posts/old.html");
        std::fs::create_dir_all(final_output.parent().unwrap()).unwrap();
        std::fs::create_dir_all(stage_output.parent().unwrap()).unwrap();
        std::fs::write(&final_output, "old").unwrap();
        std::fs::write(&stage_output, "old").unwrap();

        write_manifest(
            &paths,
            &[
                ManagedOutput {
                    group: ManagedGroup::AdocHtml,
                    path: final_output.clone(),
                },
                ManagedOutput {
                    group: ManagedGroup::AdocStage,
                    path: stage_output.clone(),
                },
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        cleanup_stale_outputs(
            &paths,
            &empty_planned([]),
            &BuildSelection::new([BuildKind::Adoc]),
        )
        .unwrap();

        assert!(!final_output.exists());
        assert!(!stage_output.exists());
    }

    #[test]
    fn cleanup_stale_outputs_keeps_paths_still_managed_by_another_group() {
        let root = temp_root();
        let repo = root.path();
        let paths = repo_paths(repo);
        let shared_output = paths.public_dir.join("resource/shape.svg");
        std::fs::create_dir_all(shared_output.parent().unwrap()).unwrap();
        std::fs::write(&shared_output, "shape").unwrap();

        write_manifest(
            &paths,
            &[ManagedOutput {
                group: ManagedGroup::Typst,
                path: shared_output.clone(),
            }]
            .into_iter()
            .collect(),
        )
        .unwrap();

        cleanup_stale_outputs(
            &paths,
            &empty_planned([ManagedOutput {
                group: ManagedGroup::Svg,
                path: shared_output.clone(),
            }]),
            &BuildSelection::new([BuildKind::Typst]),
        )
        .unwrap();

        assert!(shared_output.exists());
    }

    #[test]
    fn write_manifest_rejects_paths_outside_repo_root() {
        let root = temp_root();
        let repo = root.path();
        let paths = repo_paths(repo);

        let result = write_manifest(
            &paths,
            &[ManagedOutput {
                group: ManagedGroup::Static,
                path: repo.join("../preview/index.html"),
            }]
            .into_iter()
            .collect(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn read_manifest_ignores_paths_outside_repo_root() {
        let root = temp_root();
        let repo = root.path();
        let paths = repo_paths(repo);
        std::fs::create_dir_all(paths.cache_dir.clone()).unwrap();
        std::fs::write(
            paths.manifest_path(),
            "static\t../preview/index.html\nstatic\tpublic/index.html\n",
        )
        .unwrap();

        let outputs = read_manifest(&paths).unwrap();

        assert_eq!(outputs.len(), 1);
        assert!(outputs.iter().all(|entry| paths.is_under_root(&entry.path)));
    }
}
