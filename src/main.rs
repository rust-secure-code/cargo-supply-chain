//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use std::{error::Error, ffi::OsString, time::Duration};

use pico_args::Arguments;

mod api_client;
mod common;
mod crates_cache;
mod publishers;
mod subcommands;

/// CLI-focused help message for displaying to the user
pub(crate) const CLI_HELP: &str =
    "Usage: cargo supply-chain COMMAND [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

Commands:
  publishers   List all crates.io publishers in the depedency graph
  crates       List all crates in dependency graph and crates.io publishers for each
  json         Like 'crates', but in JSON and with more fields for each publisher
  update       Download the latest daily dump from crates.io to speed up other commands

See 'cargo supply-chain help <command>' for more information on a specific command.

Arguments:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.
  -d, --diffable   Make output more friendly towards tools such as `diff`

Any arguments after the `--` will be passed to `cargo metadata`, for example:
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.";

#[derive(Debug)]
struct Args {
    help: bool,
    command: String,
    diffable: bool,
    cache_max_age: Duration,
    metadata_args: Vec<String>,
    free: Vec<String>,
}

fn main() -> Result<(), std::io::Error> {
    match get_args() {
        Err(e) => {
            eprintln!("Error: {}", e);
            eprint_help();
            std::process::exit(1);
        }
        Ok(args) => dispatch_command(args),
    }
}

fn get_args() -> Result<ValidatedArgs, Box<dyn Error>> {
    let args = parse_args()?;
    let valid_args = validate_args(args)?;
    Ok(valid_args)
}

enum ValidatedArgs {
    Publishers {
        cache_max_age: Duration,
        diffable: bool,
        metadata_args: Vec<String>,
    },
    Crates {
        cache_max_age: Duration,
        diffable: bool,
        metadata_args: Vec<String>,
    },
    Json {
        cache_max_age: Duration,
        diffable: bool,
        metadata_args: Vec<String>,
    },
    Update {
        cache_max_age: Duration,
    },
    Help {
        command: Option<String>,
    },
}

fn validate_args(args: Args) -> Result<ValidatedArgs, std::io::Error> {
    if args.help {
        return Ok(ValidatedArgs::Help {
            command: Some(args.command),
        });
    } else {
        if args.command != "help" && !args.free.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unrecognized argument: {}", args.free[0]),
            ));
        }
        match args.command.as_str() {
            "publishers" => {
                return Ok(ValidatedArgs::Publishers {
                    cache_max_age: args.cache_max_age,
                    diffable: args.diffable,
                    metadata_args: args.metadata_args,
                })
            }
            "crates" => {
                return Ok(ValidatedArgs::Crates {
                    cache_max_age: args.cache_max_age,
                    diffable: args.diffable,
                    metadata_args: args.metadata_args,
                })
            }
            "json" => {
                return Ok(ValidatedArgs::Json {
                    cache_max_age: args.cache_max_age,
                    diffable: args.diffable,
                    metadata_args: args.metadata_args,
                })
            }
            "update" => {
                return Ok(ValidatedArgs::Update {
                    cache_max_age: args.cache_max_age,
                })
            }
            "help" => {
                return Ok(ValidatedArgs::Help {
                    command: args.free.get(0).map(String::to_owned),
                })
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unrecognized argument: {}", args.command.as_str()),
                ))
            }
        }
    }
}

fn dispatch_command(args: ValidatedArgs) -> Result<(), std::io::Error> {
    match args {
        ValidatedArgs::Publishers {
            cache_max_age,
            diffable,
            metadata_args,
        } => subcommands::publishers(metadata_args, diffable, cache_max_age)?,
        ValidatedArgs::Crates {
            cache_max_age,
            diffable,
            metadata_args,
        } => subcommands::crates(metadata_args, diffable, cache_max_age)?,
        ValidatedArgs::Json {
            cache_max_age,
            diffable,
            metadata_args,
        } => subcommands::json(metadata_args, diffable, cache_max_age)?,
        ValidatedArgs::Update { cache_max_age } => subcommands::update(cache_max_age),
        ValidatedArgs::Help { command } => subcommands::help(command.as_deref()),
    }

    Ok(())
}

fn parse_max_age(text: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(&text)
}

/// Separates arguments intended for us and for cargo-metadata
fn separate_metadata_args() -> (Vec<OsString>, Vec<String>) {
    // Everything before "--" should be parsed, and everything after it should be passed to cargo-metadata
    let mut supply_args: Vec<OsString> = std::env::args_os()
        .skip(1) // skip argv[0], the name of the binary
        .take_while(|x| x != "--")
        .collect();
    let metadata_args = std::env::args()
        .skip(1) // skip argv[0], the name of the binary
        .skip_while(|x| x != "--")
        .skip(1) // skips "--" itself
        .collect();
    // When invoked via `cargo supply-chain update`, Cargo passes the arguments it receives verbatim.
    // So instead of "update" our binary receives "supply-chain update".
    // We ignore the "supply-chain" in the beginning if it's present.
    if supply_args.get(0) == Some(&OsString::from("supply-chain")) {
        supply_args.remove(0);
    }

    (supply_args, metadata_args)
}

/// Converts all recognized arguments into a struct.
/// Does not check whether the argument is valid for the given subcommand.
fn parse_args() -> Result<Args, pico_args::Error> {
    let (supply_args, metadata_args) = separate_metadata_args();
    let default_cache_max_age = Duration::from_secs(48 * 3600);
    let mut args = Arguments::from_vec(supply_args);
    if let Some(command) = args.subcommand()? {
        let args = Args {
            help: args.contains(["-h", "--help"]),
            command,
            diffable: args.contains(["-d", "--diffable"]),
            metadata_args,
            cache_max_age: args
                .opt_value_from_fn("--cache-max-age", parse_max_age)?
                .unwrap_or(default_cache_max_age),
            free: args.free()?,
        };
        Ok(args)
    } else {
        Err(pico_args::Error::ArgumentParsingFailed {
            cause: "No subcommand given".to_string(),
        })
    }
}

fn eprint_help() {
    eprintln!("{}", CLI_HELP);
}

// TODO: remove all uses of this and return error from the function instead
pub(crate) fn err_exit(msg: &str) -> ! {
    match msg.into() {
        Some(v) => eprintln!("{}", v),
        None => (),
    };

    std::process::exit(1)
}
