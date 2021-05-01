//! Displays help infomation to the user when requested

use crate::CLI_HELP;
use std::process;
use schemars::schema_for;
use crate::subcommands::json::StructuredOutput;

/// Provides help infomation which proceeds to exit
pub fn help(command: Option<&str>) {
    match command {
        None => println!("{}", CLI_HELP),
        Some("publishers") => println!("{}", PUBLISHERS_HELP),
        Some("crates") => println!("{}", CRATES_HELP),
        Some("update") => println!("{}", UPDATE_HELP),
        Some("json") => {
            println!("{}", JSON_HELP);
            println!("{}", serde_json::to_string_pretty(&schema_for!(StructuredOutput)).unwrap());
        }
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
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.";

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
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.";

const JSON_HELP: &str = "Detailed info on publishers of all crates in the dependency graph, in JSON

The JSON schema is provided below, but the output is designed to be self-explanatory.

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.

It's not guaranteed that the local cache will be used if '--cache-max-age' is
set to less than 48 hours, even if you've run 'update' subcommand just now.
That's because crates.io database dumps may not be updated every single day.

Note that detailed information on the origin of crates outside of crates.io is not
provided. You can obtain this info from 'cargo metadata' that ships with Cargo,
or use 'cargo deny' to define a custom policy regarding crate sources.

USAGE:
  cargo supply-chain json [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

OPTIONS:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.

Any arguments after the `--` will be passed to `cargo metadata`, for example:
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.

The JSON schema definition is as follows:";

const UPDATE_HELP: &str = "Download the latest daily dump from crates.io to speed up other commands

If the local cache is already younger than specified in '--cache-max-age' option,
a newer version will not be downloaded.

Note that this downloads the entire crates.io database, which is hundreds of Mb of data!
If you are on a metered connection, you should not be running the 'update' subcommand.
Instead, rely on requests to the live API - they are slower, but use much less data.

USAGE:
  cargo supply-chain update [OPTIONS...]

OPTIONS:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.\n";