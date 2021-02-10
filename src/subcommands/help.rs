//! Displays help infomation to the user when requested

use crate::CLI_HELP;
use std::process;

/// Provides help infomation which proceeds to exit
pub fn help(command: Option<&str>) {
    match command {
        None => println!("{}", CLI_HELP),
        Some("crates") => println!("{}", CRATES_HELP),
        Some("publishers") => println!("{}", PUBLISHERS_HELP),
        Some("update") => println!("{}", UPDATE_HELP),
        Some(command) => {
            println!("Unknown subcommand: {}\n", command);
            println!("{}", CLI_HELP);
            process::exit(1)
        }
    }
    process::exit(0)
}

const CRATES_HELP: &str = "Lists all crates in dependency graph and crates.io publishers for each

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.

It's not guaranteed that the local cache will be used if '--cache-max-age' is
set to less than 48 hours, even if you've run 'update' subcommand just now.
That's because crates.io database dumps may not be updated every single day.

USAGE:
  cargo supply-chain crates [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

OPTIONS:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.

Any arguments after the `--` will be passed to `cargo metadata`, for example:
cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu\n";

const PUBLISHERS_HELP: &str =
    "Lists all crates.io publishers in the depedency graph and owned crates for each

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.

It's not guaranteed that the local cache will be used if '--cache-max-age' is
set to less than 48 hours, even if you've run 'update' subcommand just now.
That's because crates.io database dumps may not be updated every single day.

USAGE:
  cargo supply-chain publishers [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

OPTIONS:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.

Any arguments after the `--` will be passed to `cargo metadata`, for example:
cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu\n";

const UPDATE_HELP: &str = "Download the latest daily dump from crates.io to speed up other commands

If the local cache is already younger than specified in '--cache-max-age' option,
a newer version will not be downloaded.

USAGE:
  cargo supply-chain update [OPTIONS...]

OPTIONS:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.\n";
