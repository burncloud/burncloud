use burncloud_loops::cli::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let code = burncloud_loops::cli::run(cli)?;
    std::process::exit(code);
}
