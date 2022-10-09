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
        default_value = ".crabby_changelog/changelog.toml"
    )]
    pub state: PathBuf,

    // /// Turn debugging information on
    // #[arg(short, long, action = clap::ArgAction::Count)]
    // pub verbose: u8,
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
    #[arg(long, conflicts_with = "ids", conflicts_with = "since_ref")]
    pub since_timestamp: Option<u64>,
    #[arg(long, conflicts_with = "ids", conflicts_with = "since_timestamp")]
    pub since_ref: Option<String>,
    #[arg(conflicts_with = "since_ref", conflicts_with = "since_timestamp")]
    pub prs: Vec<u64>,
}
