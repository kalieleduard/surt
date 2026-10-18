mod builtin;
mod diagnostics;
mod failure;
mod feature;
mod init;
mod message;
mod refusal;
mod status;

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
    Status,
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
        Command::Status => status::run(&cwd).await,
    }
}
