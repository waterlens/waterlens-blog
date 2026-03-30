use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "wblog")]
#[command(about = "Waterlens blog build tool")]
#[command(
    long_about = "Waterlens blog build tool.\n\nBy default `build` runs incrementally and only rebuilds dirty targets. Use `clean` to remove outputs and cache, `build --full` to force the selected targets to rerun, and `--only` to limit work to specific target kinds."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Incrementally build the site or selected target kinds")]
    Build(BuildArgs),
    #[command(about = "Remove generated outputs and the local build cache")]
    Clean,
    #[command(about = "Serve the generated site from the output directory")]
    Serve(ServeArgs),
    #[command(about = "Watch source files, rebuild affected targets, and serve the site")]
    Watch(WatchArgs),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum BuildKind {
    Typst,
    Adoc,
    Css,
    Svg,
    Static,
    Tidy,
}

impl BuildKind {
    pub const ALL: [Self; 6] = [
        Self::Typst,
        Self::Adoc,
        Self::Css,
        Self::Svg,
        Self::Static,
        Self::Tidy,
    ];
}

#[derive(Clone, Debug, Default, Args)]
pub struct BuildFilterArgs {
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Limit work to specific target kinds, for example `--only typst,adoc`"
    )]
    pub only: Vec<BuildKind>,
}

#[derive(Clone, Debug, Args)]
pub struct BuildArgs {
    #[command(flatten)]
    pub filter: BuildFilterArgs,

    #[arg(
        long,
        help = "Force the selected targets to rerun even if they look up to date"
    )]
    pub full: bool,
}

#[derive(Clone, Debug, Args)]
pub struct ServeArgs {
    #[arg(
        long,
        default_value = "::1",
        help = "Bind address for the local HTTP server"
    )]
    pub host: String,

    #[arg(long, default_value_t = 8668, help = "Port for the local HTTP server")]
    pub port: u16,

    #[arg(long, default_value = "public", help = "Directory to serve")]
    pub root: PathBuf,
}

#[derive(Clone, Debug, Args)]
pub struct WatchServeArgs {
    #[arg(
        long,
        default_value = "::1",
        help = "Bind address for the local HTTP server"
    )]
    pub host: String,

    #[arg(long, default_value_t = 8668, help = "Port for the local HTTP server")]
    pub port: u16,

    #[arg(long, help = "Directory to serve [default: build output directory]")]
    pub root: Option<PathBuf>,
}

#[derive(Clone, Debug, Args)]
pub struct WatchArgs {
    #[command(flatten)]
    pub filter: BuildFilterArgs,

    #[arg(
        long,
        help = "Directory to write generated output into [default: public]"
    )]
    pub output_root: Option<PathBuf>,

    #[command(flatten)]
    pub serve: WatchServeArgs,
}
