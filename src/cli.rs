use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Path to configuration file
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = ".crabby_changelog/config.toml"
    )]
    pub config: PathBuf,

    /// Path to configuration file
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = ".crabby_changelog/changelog.json"
    )]
    pub state: PathBuf,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Render(Render),
    AddPr(AddPr),
}

#[derive(Args)]
pub struct Render {
    pub version: Option<String>,
}

#[derive(Args)]
pub struct AddPr {
    /// The repository that the PR belongs to
    /// like `pascalkuthe/crabby_changelog`
    pub repo: String,
    /// The nubmer that is used to refer to the PR
    /// (in markdown using `#id`).
    pub ids: Vec<u32>,
}
