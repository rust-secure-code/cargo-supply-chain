//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use common::bail_bad_arg;

mod api_client;
mod authors;
mod common;
mod crates_cache;
mod publishers;
mod subcommands;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.by_ref().next();

    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("supply-chain") => (), // first arg when run as `cargo supply-chain`
            Some("authors") => return subcommands::authors(args),
            Some("publishers") => return subcommands::publishers(args),
            Some("crates") => return subcommands::crates(args),
            Some("update") => return subcommands::update(args),
            Some(arg) if arg.starts_with("--") => bail_unknown_option(arg),
            Some(arg) if arg.starts_with('-') => bail_unknown_short_option(arg),
            Some(arg) => bail_unknown_command(arg),
        }
    }

    // No command selected.
    bail_no_command();
}

fn bail_unknown_option(arg: &str) -> ! {
    eprintln!("Unknown option: {}", std::path::Path::new(&arg).display());
    eprint_help();
    std::process::exit(1);
}

fn bail_unknown_short_option(arg: &str) -> ! {
    eprintln!("Unknown flag: {}", arg);
    eprint_help();
    std::process::exit(1);
}

fn bail_unknown_command(arg: &str) -> ! {
    eprintln!("Unknown command: {}", arg);
    eprint_help();
    std::process::exit(1);
}

fn bail_no_command() -> ! {
    eprintln!("No command selected.");
    eprint_help();
    std::process::exit(1);
}

fn eprint_help() {
    eprintln!(
        "Usage: cargo supply-chain COMMAND [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

  Commands:
    authors\t\tList all authors in the dependency graph (as specified in Cargo.toml)
    publishers\t\tList all crates.io publishers in the dependency graph
    crates\t\tList all crates in dependency graph and crates.io publishers for each
    update\t\tDownload the latest daily dump from crates.io to speed up other commands
  Arguments:
    --cache-max-age\t\tThe cache will be considered valid while younger than specified.
    \t\t\tThe format is a human recognizable duration such as `1w` or `1d 6h`.

  Any arguments after -- will be passed to `cargo metadata`, for example:
    cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
"
    );
}
