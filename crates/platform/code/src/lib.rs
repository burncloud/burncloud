//! Repository development commands; no dependency on application services.
use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

mod init;
mod plan;
mod report;
mod test;

/// Shared command definition for the application and the lightweight tooling binary.
pub fn command() -> Command {
    Command::new("code")
        .about("Initialize and test the source-code development environment")
        .subcommand_required(true)
        .subcommand(Command::new("init").about("Install local pre-commit quality checks"))
        .subcommand(
            Command::new("test")
                .about("Check changed packages and their transitive workspace consumers")
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(ArgAction::SetTrue)
                        .help("Check the entire workspace"),
                )
                .arg(
                    Arg::new("plan")
                        .long("plan")
                        .action(ArgAction::SetTrue)
                        .help("Print selection and commands without running checks"),
                )
                .arg(
                    Arg::new("staged")
                        .long("staged")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("base")
                        .help("Check staged changes; reject unstaged and untracked files"),
                )
                .arg(Arg::new("base").long("base").value_name("REF").help(
                    "Include branch changes since the merge base with REF, plus local changes",
                ))
                .arg(Arg::new("last").long("last").action(ArgAction::SetTrue)
                    .conflicts_with_all(["all", "plan", "staged", "base"])
                    .help("Show the latest saved check results and log paths")),
        )
        .subcommand(Command::new("stamp").hide(true)
            .arg(Arg::new("message").required(true)))
}

/// Execute a parsed `code` subcommand.
pub fn handle(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("init", _)) => Ok(init::init()?),
        Some(("test", options)) => test::run(test::Options {
            all: options.get_flag("all"),
            plan_only: options.get_flag("plan"),
            staged: options.get_flag("staged"),
            last: options.get_flag("last"),
            base: options.get_one::<String>("base").cloned(),
        }),
        Some(("stamp", options)) => report::stamp(std::path::Path::new(
            options.get_one::<String>("message").expect("required message"))),
        _ => anyhow::bail!("Expected code init or code test"),
    }
}
