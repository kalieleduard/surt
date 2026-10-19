mod cancel;
mod confirmation;
mod decide;
mod delete;
mod diagnostics;
mod driving;
mod duration;
mod failure;
mod feature;
mod init;
mod inspect;
mod json;
mod logs;
mod merge_flag;
mod message;
mod phase;
mod plan;
mod refusal;
mod restore;
mod retry;
mod run;
mod runtime;
mod show;
mod start;
mod states;
mod status;
mod stop;
mod symbols;
mod table;
mod template;

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
    Init(init::InitArgs),
    Run(run::RunArgs),
    Feature(feature::FeatureArgs),
    Plan(plan::PlanArgs),
    Start(start::StartArgs),
    Approve(decide::ApproveArgs),
    Reject(decide::RejectArgs),
    Cancel(cancel::CancelArgs),
    Delete(delete::DeleteArgs),
    Restore(restore::RestoreArgs),
    Status(status::StatusArgs),
    Stop(stop::StopArgs),
    Show(show::ShowArgs),
    Inspect(inspect::InspectArgs),
    Logs(logs::LogsArgs),
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
        Command::Init(args) => init::run(&cwd, args).await,
        Command::Run(args) => run::run(&cwd, args).await,
        Command::Feature(args) => feature::run(&cwd, args).await,
        Command::Plan(args) => plan::run(&cwd, args).await,
        Command::Start(args) => start::run(&cwd, args).await,
        Command::Approve(args) => decide::approve(&cwd, args).await,
        Command::Reject(args) => decide::reject(&cwd, args).await,
        Command::Cancel(args) => cancel::run(&cwd, args).await,
        Command::Delete(args) => delete::run(&cwd, args).await,
        Command::Restore(args) => restore::run(&cwd, args).await,
        Command::Status(args) => status::run(&cwd, args).await,
        Command::Stop(args) => stop::run(&cwd, args).await,
        Command::Show(args) => show::run(&cwd, args).await,
        Command::Inspect(args) => inspect::run(&cwd, args).await,
        Command::Logs(args) => logs::run(&cwd, args).await,
    }
}
