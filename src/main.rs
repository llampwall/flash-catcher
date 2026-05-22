mod admin;
mod aggregate;
mod blame;
mod classify;
mod cli;
mod etw;
mod event;
mod process;
mod store;
mod web;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Run(args) => run_collector(args).await,
        Command::View(args) => run_viewer(args).await,
        Command::ClassifyRules(args) => print_classify_rules(args),
    }
}

async fn run_collector(_args: cli::RunArgs) -> Result<()> {
    unimplemented!("collector entry point: admin gate -> ETW session -> store -> web server")
}

async fn run_viewer(_args: cli::ViewArgs) -> Result<()> {
    unimplemented!("viewer-only mode: read existing JSONL, serve web UI without ETW capture")
}

fn print_classify_rules(_args: cli::ClassifyRulesArgs) -> Result<()> {
    unimplemented!("dump active classification rules as JSON for inspection")
}
