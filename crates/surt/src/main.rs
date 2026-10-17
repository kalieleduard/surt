use clap::Parser;

#[derive(Parser)]
#[command(name = "surt", version)]
struct Cli {}

#[tokio::main]
async fn main() {
    Cli::parse();
}
