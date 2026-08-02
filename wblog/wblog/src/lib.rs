mod build_graph;
mod cli;
mod commands;
mod console_progress;
mod paths;
mod serve;
mod tools;
mod watch;

pub mod output;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use commands::Context;
use paths::RepoPaths;
use tools::ToolResolver;

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build(args) => {
            let paths = RepoPaths::from_current_dir()?;
            let tools = ToolResolver::from_env();
            let context = Context::new(paths, tools);
            context.build_from_args(&args)
        }
        Command::Clean => {
            let paths = RepoPaths::from_current_dir()?;
            let tools = ToolResolver::from_env();
            let context = Context::new(paths, tools);
            context.clean()
        }
        Command::Serve(args) => serve::serve(args).await,
        Command::Watch(args) => {
            let paths = RepoPaths::from_current_dir()?;
            let tools = ToolResolver::from_env();
            let context = Context::new(paths, tools);
            watch::watch(context, args).await
        }
    }
}
