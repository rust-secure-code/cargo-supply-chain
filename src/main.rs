//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use std::{path::PathBuf, time::Duration};

use bpaf::*;

mod api_client;
mod common;
mod crates_cache;
mod publishers;
mod subcommands;

fn main() -> Result<(), std::io::Error> {
    let args = args_parser().run();
    dispatch_command(args)
}

fn args_parser() -> OptionParser<CliArgs> {
    let diffable = short('d')
        .long("diffable")
        .help("Make output more friendly towards tools such as `diff`")
        .switch();
    let cache_max_age_parser = long("cache-max-age")
        .help(
            "\
The cache will be considered valid while younger than specified.
The format is a human readable duration such as `1w` or `1d 6h`.
If not specified, the cache is considered valid for 48 hours.",
        )
        .argument("AGE")
        .parse(|text| humantime::parse_duration(&text))
        .fallback(Duration::from_secs(48 * 3600));
    let cache_max_age = cache_max_age_parser.clone();
    let args_parser = construct!(QueryCommandArgs {
        cache_max_age,
        diffable,
    });

    let all_features = long("all-features")
        .help("Activate all available features")
        .switch();
    let no_default_features = long("no-default-features")
        .help("Do not activate the `default` feature")
        .switch();
    let features = long("features")
        .help("Space or comma separated list of features to activate")
        .argument("FEATURES")
        .optional();
    let target = long("target")
        .help("Only include dependencies matching the given target-triple")
        .argument("TRIPLE")
        .optional();
    let manifest_path = long("manifest-path")
        .help("Path to Cargo.toml")
        .argument_os("PATH")
        .map(|s| PathBuf::from(s))
        .optional();

    let metadata_args_parser = construct!(MetadataArgs {
        all_features,
        no_default_features,
        features,
        target,
        manifest_path,
    });

    fn subcommand_with_common_args(
        command_name: &'static str,
        args: Parser<QueryCommandArgs>,
        meta_args: Parser<MetadataArgs>,
        descr: &'static str,
        descr_ext: &'static str,
    ) -> bpaf::Parser<CliArgs> {
        let parser = match command_name {
            "publishers" => construct!(CliArgs::Publishers { args, meta_args }),
            "crates" => construct!(CliArgs::Crates { args, meta_args }),
            "json" => construct!(CliArgs::Json { args, meta_args }),
            _ => unreachable!(),
        };
        let parser = Info::default().descr(descr_ext).for_parser(parser);
        command(command_name, Some(descr), parser)
    }

    let publishers = subcommand_with_common_args(
        "publishers",
        args_parser.clone(),
        metadata_args_parser.clone(),
        "List all crates.io publishers in the depedency graph",
        "Lists all crates.io publishers in the dependency graph and owned crates for each

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.",
    );
    let crates = subcommand_with_common_args(
        "crates",
        args_parser.clone(),
        metadata_args_parser.clone(),
        "List all crates in dependency graph and crates.io publishers for each",
        "Lists all crates in dependency graph and crates.io publishers for each

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.",
    );
    let json = subcommand_with_common_args(
        "json",
        args_parser.clone(),
        metadata_args_parser.clone(),
        "Like 'crates', but in JSON and with more fields for each publisher",
        "Detailed info on publishers of all crates in the dependency graph, in JSON

The JSON schema is provided below, but the output is designed to be self-explanatory.

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.",
    );

    let cache_max_age = cache_max_age_parser.clone();
    let update = construct!(CliArgs::Update { cache_max_age });
    let update = Info::default()
        .descr(
            "Download the latest daily dump from crates.io to speed up other commands

If the local cache is already younger than specified in '--cache-max-age' option,
a newer version will not be downloaded.
        
Note that this downloads the entire crates.io database, which is hundreds of Mb of data!
If you are on a metered connection, you should not be running the 'update' subcommand.
Instead, rely on requests to the live API - they are slower, but use much less data.",
        )
        .for_parser(update);
    let update = command(
        "update",
        Some("Download the latest daily dump from crates.io to speed up other commands"),
        update,
    );

    let parser = publishers.or_else(crates).or_else(json).or_else(update);

    Info::default()
        .version(env!("CARGO_PKG_VERSION"))
        .descr("Gather author, contributor and publisher data on crates in your dependency graph")
        .for_parser(parser)
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
        CliArgs::Update { cache_max_age } => subcommands::update(cache_max_age),
    }

    Ok(())
}

/// Arguments for typical querying commands - crates, publishers, json
#[derive(Clone, Debug)]
struct QueryCommandArgs {
    cache_max_age: Duration,
    diffable: bool,
}

#[derive(Clone, Debug)]
enum CliArgs {
    Publishers {
        args: QueryCommandArgs,
        meta_args: MetadataArgs,
    },
    Crates {
        args: QueryCommandArgs,
        meta_args: MetadataArgs,
    },
    Json {
        args: QueryCommandArgs,
        meta_args: MetadataArgs,
    },
    Update {
        cache_max_age: Duration,
    },
}

/// Arguments to be passed to `cargo metadata`
#[derive(Clone, Debug)]
pub struct MetadataArgs {
    // `all_features` and `no_default_features` are not mutually exclusive in `cargo metadata`,
    // in the sense that it will not error out when encontering them; it just follows `all_features`
    all_features: bool,
    no_default_features: bool,
    // This is a `String` because we don't parse the value, just pass it on to `cargo metadata` blindly
    features: Option<String>,
    target: Option<String>,
    manifest_path: Option<PathBuf>,
}

// TODO: remove all uses of this and return error from the function instead
pub(crate) fn err_exit(msg: &str) -> ! {
    match msg.into() {
        Some(v) => eprintln!("{}", v),
        None => (),
    };

    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_max_age_parser() {
        let _ = args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age", "7d"]))
            .unwrap();
        let _ = args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age=7d"]))
            .unwrap();
        let _ = args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age=1w"]))
            .unwrap();
        let _ = args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age=1m"]))
            .unwrap();
        let _ = args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age=1s"]))
            .unwrap();
        // erroneous invocations that must be rejected
        assert!(args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age"]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&["crates", "--cache-max-age=5"]))
            .is_err());
    }

    #[test]
    fn test_accepted_query_options() {
        for command in ["crates", "publishers", "json"] {
            let _ = args_parser().run_inner(Args::from(&[command])).unwrap();
            let _ = args_parser()
                .run_inner(Args::from(&[command, "-d"]))
                .unwrap();
            let _ = args_parser()
                .run_inner(Args::from(&[command, "--diffable"]))
                .unwrap();
            let _ = args_parser()
                .run_inner(Args::from(&[command, "--cache-max-age=7d"]))
                .unwrap();
            let _ = args_parser()
                .run_inner(Args::from(&[command, "-d", "--cache-max-age=7d"]))
                .unwrap();
            let _ = args_parser()
                .run_inner(Args::from(&[command, "--diffable", "--cache-max-age=7d"]))
                .unwrap();
        }
    }

    #[test]
    fn test_accepted_update_options() {
        let _ = args_parser().run_inner(Args::from(&["update"])).unwrap();
        let _ = args_parser()
            .run_inner(Args::from(&["update", "--cache-max-age=7d"]))
            .unwrap();
        // erroneous invocations that must be rejected
        assert!(args_parser()
            .run_inner(Args::from(&["update", "-d"]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&["update", "--diffable"]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&["update", "-d", "--cache-max-age=7d"]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&["update", "--diffable", "--cache-max-age=7d"]))
            .is_err());
    }
}
