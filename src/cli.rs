use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "flash-watcher", about = "Windows console-flash investigator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run ETW collector + web UI (requires admin)
    Run(RunArgs),
    /// View-only: serve UI against existing JSONL without ETW capture
    View(ViewArgs),
    /// Print the active classification rules
    ClassifyRules(ClassifyRulesArgs),
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// HTTP server bind address
    #[arg(long, default_value = "127.0.0.1:7790")]
    pub bind: String,

    /// JSONL store directory
    #[arg(long, default_value = "data")]
    pub data_dir: PathBuf,

    /// Skip the admin elevation check (collector will fail without it; for testing only)
    #[arg(long)]
    pub skip_admin_check: bool,

    /// Open the browser automatically after the server starts
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Args)]
pub struct ViewArgs {
    #[arg(long, default_value = "127.0.0.1:7790")]
    pub bind: String,
    #[arg(long, default_value = "data")]
    pub data_dir: PathBuf,
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Args)]
pub struct ClassifyRulesArgs {
    /// Output as pretty JSON instead of compact
    #[arg(long)]
    pub pretty: bool,
}
