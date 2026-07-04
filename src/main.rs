use clap::Parser;
use ibex_benchmark_bot::cli;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    if let Err(err) = cli::run(cli).await {
        eprintln!("ibex-benchmark-bot: {err}");
        std::process::exit(1);
    }
}
