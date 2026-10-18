mod builtin;
mod cancel;
mod decide;
mod diagnostics;
mod driving;
mod failure;
mod feature;
mod init;
mod message;
mod phase;
mod plan;
mod refusal;
mod runtime;
mod show;
mod start;
mod status;
mod table;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use failure::Failure;

#[derive(Parser)]
#[command(name = "surt", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init,
    Feature(feature::FeatureArgs),
    Plan(plan::PlanArgs),
    Start(start::StartArgs),
    Approve(decide::ApproveArgs),
    Reject(decide::RejectArgs),
    Cancel(cancel::CancelArgs),
    Status(status::StatusArgs),
    Show(show::ShowArgs),
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    diagnostics::install();
    match run(cli.command).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}

async fn run(command: Command) -> Result<(), Failure> {
    let cwd = std::env::current_dir()?;
    match command {
        Command::Init => init::run(&cwd).await,
        Command::Feature(args) => feature::run(&cwd, args).await,
        Command::Plan(args) => plan::run(&cwd, args).await,
        Command::Start(args) => start::run(&cwd, args).await,
        Command::Approve(args) => decide::approve(&cwd, args).await,
        Command::Reject(args) => decide::reject(&cwd, args).await,
        Command::Cancel(args) => cancel::run(&cwd, args).await,
        Command::Status(args) => status::run(&cwd, args).await,
        Command::Show(args) => show::run(&cwd, args).await,
    }
}
