//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

mod api_client;
mod cli;
mod common;
mod crates_cache;
mod publishers;
mod subcommands;

use cli::CliArgs;
use common::MetadataArgs;

fn main() -> Result<(), std::io::Error> {
    let args = cli::args_parser().run();
    dispatch_command(args)
}

fn dispatch_command(args: CliArgs) -> Result<(), std::io::Error> {
    match args {
        CliArgs::Publishers { args, meta_args } => {
            subcommands::publishers(meta_args, args.diffable, args.cache_max_age)?
        }
        CliArgs::Crates { args, meta_args } => {
            subcommands::crates(meta_args, args.diffable, args.cache_max_age)?
        }
        CliArgs::Json { args, meta_args } => {
            subcommands::json(meta_args, args.diffable, args.cache_max_age)?
        }
        CliArgs::JsonSchema { print_schema: () } => {
            subcommands::print_schema()?;
        }
        CliArgs::Update { cache_max_age } => subcommands::update(cache_max_age),
        CliArgs::Lines { meta_args } => subcommands::lines(meta_args),
    }

    Ok(())
}

// TODO: remove all uses of this and return error from the function instead
pub(crate) fn err_exit(msg: &str) -> ! {
    match msg.into() {
        Some(v) => eprintln!("{}", v),
        None => (),
    };

    std::process::exit(1)
}
