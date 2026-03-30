use std::time::Duration;

use anyhow::{Result, bail};
use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::task::JoinHandle;

use crate::build_graph::ChangeScope;
use crate::cli::{BuildFilterArgs, BuildKind, ServeArgs, WatchArgs, WatchServeArgs};
use crate::commands::Context;
use crate::output;
use crate::serve;

pub async fn watch(mut context: Context, args: WatchArgs) -> Result<()> {
    let output_root = resolve_output_root(&context, args.output_root.as_deref())?;
    context.paths.public_dir = output_root.clone();
    let serve_args = resolve_serve_args(&context, &args.serve, &output_root);

    run_initial_build(context.clone(), args.filter.clone()).await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(300), move |result| {
        let _ = tx.send(result);
    })?;

    for (path, mode) in watch_targets(&context, &args.filter) {
        debouncer.watcher().watch(&path, mode)?;
    }

    let mut server = tokio::spawn(serve::serve(serve_args));
    let mut build = None::<JoinHandle<Result<()>>>;
    let mut pending = ChangeScope::default();

    loop {
        tokio::select! {
            result = &mut server => {
                match result {
                    Ok(inner) => inner?,
                    Err(error) => return Err(error.into()),
                }
                return Ok(());
            }
            result = async {
                let task = build.as_mut().expect("build branch should only run when active");
                task.await
            }, if build.is_some() => {
                build = None;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        eprintln!(
                            "{} build failed: {error:#}",
                            output::tag_stderr("watch", output::YELLOW),
                        );
                    }
                    Err(error) => return Err(error.into()),
                }

                if !pending.is_empty() {
                    let scope = take_pending_scope(&context.paths, &mut rx, &mut pending);
                    build = Some(start_build(context.clone(), args.filter.clone(), scope));
                }
            }
            maybe_result = rx.recv() => {
                let Some(result) = maybe_result else {
                    return Ok(());
                };

                let mut scope = ChangeScope::default();
                absorb_result(&context.paths, &mut scope, result);
                if scope.is_empty() {
                    continue;
                }

                if build.is_some() {
                    pending.merge(scope);
                    continue;
                }

                drain_ready_results(&context.paths, &mut rx, &mut scope);
                build = Some(start_build(context.clone(), args.filter.clone(), scope));
            }
            signal = tokio::signal::ctrl_c() => {
                signal?;
                server.abort();
                return Ok(());
            }
        }
    }
}

fn start_build(
    context: Context,
    filter: crate::cli::BuildFilterArgs,
    changes: ChangeScope,
) -> JoinHandle<Result<()>> {
    tokio::task::spawn_blocking(move || context.build_for_changes(&filter, changes))
}

async fn run_initial_build(context: Context, filter: crate::cli::BuildFilterArgs) -> Result<()> {
    tokio::task::spawn_blocking(move || context.build_incremental(&filter)).await??;
    Ok(())
}

fn take_pending_scope(
    paths: &crate::paths::RepoPaths,
    rx: &mut mpsc::UnboundedReceiver<DebounceEventResult>,
    pending: &mut ChangeScope,
) -> ChangeScope {
    let mut scope = std::mem::take(pending);
    drain_ready_results(paths, rx, &mut scope);
    scope
}

fn drain_ready_results(
    paths: &crate::paths::RepoPaths,
    rx: &mut mpsc::UnboundedReceiver<DebounceEventResult>,
    scope: &mut ChangeScope,
) {
    loop {
        match rx.try_recv() {
            Ok(result) => absorb_result(paths, scope, result),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return,
        }
    }
}

fn absorb_result(
    paths: &crate::paths::RepoPaths,
    scope: &mut ChangeScope,
    result: DebounceEventResult,
) {
    match result {
        Ok(events) => {
            scope.merge(ChangeScope::from_events(
                paths,
                events.into_iter().map(|event| event.path),
            ));
        }
        Err(error) => {
            eprintln!("{} {error:#}", output::tag_stderr("watch", output::YELLOW),);
        }
    }
}

fn resolve_output_root(
    context: &Context,
    root: Option<&std::path::Path>,
) -> Result<std::path::PathBuf> {
    let output_root = root
        .map(|path| context.paths.resolve_rooted_path(path))
        .unwrap_or_else(|| context.paths.public_dir.clone());
    if !context.paths.is_under_root(&output_root) {
        bail!(
            "watch output root {} must stay under repo root {}",
            output_root.display(),
            context.paths.root.display()
        );
    }

    Ok(output_root)
}

fn resolve_serve_args(
    context: &Context,
    serve: &WatchServeArgs,
    output_root: &std::path::Path,
) -> ServeArgs {
    let root = serve
        .root
        .as_deref()
        .map(|path| context.paths.resolve_rooted_path(path))
        .unwrap_or_else(|| output_root.to_path_buf());
    ServeArgs {
        host: serve.host.clone(),
        port: serve.port,
        root,
    }
}

fn watch_targets(
    context: &Context,
    filter: &BuildFilterArgs,
) -> Vec<(std::path::PathBuf, RecursiveMode)> {
    let kinds = if filter.only.is_empty() {
        BuildKind::ALL.to_vec()
    } else {
        filter.only.clone()
    };

    let mut targets = Vec::new();
    for kind in kinds {
        match kind {
            BuildKind::Typst => push_watch(
                &mut targets,
                context.paths.resource_typst_dir.clone(),
                RecursiveMode::Recursive,
            ),
            BuildKind::Adoc => {
                push_watch(
                    &mut targets,
                    context.paths.content_dir.clone(),
                    RecursiveMode::Recursive,
                );
                push_watch(
                    &mut targets,
                    context.paths.w_asciidoc_dir.clone(),
                    RecursiveMode::Recursive,
                );
                push_watch(
                    &mut targets,
                    context.paths.tidy_config.clone(),
                    RecursiveMode::NonRecursive,
                );
            }
            BuildKind::Css => {
                push_watch(
                    &mut targets,
                    context.paths.sass_dir().to_path_buf(),
                    RecursiveMode::Recursive,
                );
            }
            BuildKind::Svg => push_watch(
                &mut targets,
                context.paths.resource_svg_dir.clone(),
                RecursiveMode::Recursive,
            ),
            BuildKind::Static => {
                push_watch(
                    &mut targets,
                    context.paths.static_dir.clone(),
                    RecursiveMode::Recursive,
                );
                push_watch(
                    &mut targets,
                    context.paths.tidy_config.clone(),
                    RecursiveMode::NonRecursive,
                );
            }
            BuildKind::Tidy => {
                push_watch(
                    &mut targets,
                    context.paths.content_dir.clone(),
                    RecursiveMode::Recursive,
                );
                push_watch(
                    &mut targets,
                    context.paths.static_dir.clone(),
                    RecursiveMode::Recursive,
                );
                push_watch(
                    &mut targets,
                    context.paths.w_asciidoc_dir.clone(),
                    RecursiveMode::Recursive,
                );
                push_watch(
                    &mut targets,
                    context.paths.tidy_config.clone(),
                    RecursiveMode::NonRecursive,
                );
            }
        }
    }

    targets
}

fn push_watch(
    targets: &mut Vec<(std::path::PathBuf, RecursiveMode)>,
    path: std::path::PathBuf,
    mode: RecursiveMode,
) {
    if path.exists() && !targets.iter().any(|(existing, _)| existing == &path) {
        targets.push((path, mode));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind, notify::RecursiveMode};

    use super::{
        absorb_result, drain_ready_results, push_watch, resolve_output_root, resolve_serve_args,
        watch_targets,
    };
    use crate::build_graph::ChangeScope;
    use crate::cli::{BuildFilterArgs, BuildKind, WatchServeArgs};
    use crate::commands::Context;
    use crate::paths::RepoPaths;
    use crate::tools::ToolResolver;

    fn fixture_context() -> Context {
        let repo = tempfile::tempdir().unwrap().keep();
        std::fs::create_dir_all(repo.join("content")).unwrap();
        std::fs::create_dir_all(repo.join("static")).unwrap();
        std::fs::create_dir_all(repo.join("resource/typst")).unwrap();
        std::fs::create_dir_all(repo.join("resource/svg")).unwrap();
        std::fs::create_dir_all(repo.join("tools/asciidoc")).unwrap();
        std::fs::create_dir_all(repo.join("styles")).unwrap();
        std::fs::write(repo.join("styles/style.scss"), "").unwrap();
        std::fs::write(repo.join("tidy.cfg"), "").unwrap();

        Context::new(
            RepoPaths {
                root: repo.to_path_buf(),
                public_dir: repo.join("public"),
                cache_dir: repo.join(".wblog-cache"),
                content_dir: repo.join("content"),
                static_dir: repo.join("static"),
                resource_typst_dir: repo.join("resource/typst"),
                resource_svg_dir: repo.join("resource/svg"),
                sass_style: repo.join("styles/style.scss"),
                tidy_config: repo.join("tidy.cfg"),
                w_asciidoc_dir: repo.join("tools/asciidoc"),
            },
            ToolResolver::from_env(),
        )
    }

    #[test]
    fn push_watch_deduplicates_paths() {
        let mut targets = Vec::new();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("content");
        std::fs::create_dir_all(&path).unwrap();
        push_watch(&mut targets, path.clone(), RecursiveMode::Recursive);
        push_watch(&mut targets, path.clone(), RecursiveMode::Recursive);
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn adoc_watch_only_subscribes_to_adoc_inputs() {
        let context = fixture_context();
        let targets = watch_targets(
            &context,
            &BuildFilterArgs {
                only: vec![BuildKind::Adoc],
            },
        );
        let paths = targets
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();
        assert!(paths.contains(&context.paths.content_dir));
        assert!(paths.contains(&context.paths.w_asciidoc_dir));
        assert!(paths.contains(&context.paths.tidy_config));
        assert!(!paths.contains(&context.paths.resource_typst_dir));
        assert!(!paths.contains(&context.paths.resource_svg_dir));
        assert!(!paths.contains(&context.paths.static_dir));
        assert!(!paths.contains(&context.paths.sass_dir().to_path_buf()));
    }

    #[test]
    fn resolve_output_root_rejects_paths_outside_repo_root() {
        let context = fixture_context();
        let result = resolve_output_root(&context, Some(Path::new("../preview")));
        assert!(result.is_err());
    }

    #[test]
    fn watch_serves_output_root_by_default() {
        let context = fixture_context();
        let output_root = context.paths.root.join("public-preview");
        let serve_args = resolve_serve_args(
            &context,
            &WatchServeArgs {
                host: "::1".to_owned(),
                port: 8668,
                root: None,
            },
            &output_root,
        );
        assert_eq!(serve_args.root, output_root);
    }

    #[test]
    fn absorb_result_tracks_changed_source_paths() {
        let context = fixture_context();
        let mut scope = ChangeScope::default();
        absorb_result(
            &context.paths,
            &mut scope,
            Ok(vec![DebouncedEvent::new(
                context.paths.content_dir.join("post.adoc"),
                DebouncedEventKind::Any,
            )]),
        );

        assert!(scope.tracks_source(&context.paths.content_dir.join("post.adoc")));
    }

    #[test]
    fn drain_ready_results_merges_batched_events() {
        let context = fixture_context();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tx.send(Ok(vec![DebouncedEvent::new(
            context.paths.content_dir.join("first.adoc"),
            DebouncedEventKind::Any,
        )]))
        .unwrap();
        tx.send(Ok(vec![DebouncedEvent::new(
            context.paths.static_dir.join("index.html"),
            DebouncedEventKind::AnyContinuous,
        )]))
        .unwrap();

        let mut scope = ChangeScope::default();
        drain_ready_results(&context.paths, &mut rx, &mut scope);

        assert!(scope.tracks_source(&context.paths.content_dir.join("first.adoc")));
        assert!(scope.tracks_source(&context.paths.static_dir.join("index.html")));
    }
}
