use crate::common::MetadataArgs;
use bpaf::*;
use std::{path::PathBuf, time::Duration};

/// Arguments for typical querying commands - crates, publishers, json
#[derive(Clone, Debug)]
pub(crate) struct QueryCommandArgs {
    pub cache_max_age: Duration,
    pub diffable: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum CliArgs {
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
    JsonSchema {
        print_schema: (),
    },
    Update {
        cache_max_age: Duration,
    },
}

fn cache_max_age() -> Parser<Duration> {
    long("cache-max-age")
        .help(
            "\
The cache will be considered valid while younger than specified.
The format is a human readable duration such as `1w` or `1d 6h`.
If not specified, the cache is considered valid for 48 hours.",
        )
        .argument("AGE")
        .parse(|text| humantime::parse_duration(&text))
        .fallback(Duration::from_secs(48 * 3600))
}

fn args() -> Parser<QueryCommandArgs> {
    let diffable = short('d')
        .long("diffable")
        .help("Make output more friendly towards tools such as `diff`")
        .switch();
    construct!(QueryCommandArgs {
        cache_max_age(),
        diffable,
    })
}

fn meta_args() -> Parser<MetadataArgs> {
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
        .map(PathBuf::from)
        .optional();
    construct!(MetadataArgs {
        all_features,
        no_default_features,
        features,
        target,
        manifest_path,
    })
}

pub(crate) fn args_parser() -> OptionParser<CliArgs> {
    let publishers = {
        let publishers_long =
            "Lists all crates.io publishers in the dependency graph and owned crates for each

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.";
        let publishers_short = "List all crates.io publishers in the depedency graph";
        let parser = Info::default()
            .descr(publishers_long)
            .for_parser(construct!(CliArgs::Publishers { args(), meta_args() }));
        command("publishers", Some(publishers_short), parser)
    };

    let crates = {
        let parser = construct!(CliArgs::Crates { args(), meta_args() });
        let crates_long = "Lists all crates in dependency graph and crates.io publishers for each

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.";
        let crates_short = "List all crates in dependency graph and crates.io publishers for each";
        let parser = Info::default().descr(crates_long).for_parser(parser);
        command("crates", Some(crates_short), parser)
    };

    let json = {
        let print_schema = long("print-schema")
            .help("Print JSON schema and exit")
            .req_flag(());
        let parser = construct!(CliArgs::Json { args(), meta_args() })
            .or_else(construct!(CliArgs::JsonSchema { print_schema }));

        let parser = Info::default()
            .descr(
                "Detailed info on publishers of all crates in the dependency graph, in JSON

The JSON schema is also available, use --print-schema to get it.

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.",
            )
            .for_parser(parser);
        command(
            "json",
            Some("Like 'crates', but in JSON and with more fields for each publisher"),
            parser,
        )
    };

    let update = Info::default()
        .descr(
            "Download the latest daily dump from crates.io to speed up other commands

If the local cache is already younger than specified in '--cache-max-age' option,
a newer version will not be downloaded.

Note that this downloads the entire crates.io database, which is hundreds of Mb of data!
If you are on a metered connection, you should not be running the 'update' subcommand.
Instead, rely on requests to the live API - they are slower, but use much less data.",
        )
        .for_parser(construct!(CliArgs::Update { cache_max_age() }));
    let update = command(
        "update",
        Some("Download the latest daily dump from crates.io to speed up other commands"),
        update,
    );

    let parser = cargo_helper(
        "supply-chain",
        construct!([publishers, crates, json, update]),
    );

    Info::default()
        .version(env!("CARGO_PKG_VERSION"))
        .descr("Gather author, contributor and publisher data on crates in your dependency graph")
        .footer(
            "\
Most commands also accept flags controlling the features, targets, etc.
See 'cargo supply-chain <command> --help' for more information on a specific command.",
        )
        .for_parser(parser)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn parse_args<T: AsRef<OsStr> + ?Sized>(args: &[&T]) -> Result<CliArgs, ParseFailure> {
        let args: Vec<&OsStr> = args.iter().map(|a| a.as_ref()).collect();
        let args: &[&OsStr] = &args;
        args_parser().run_inner(Args::from(args))
    }

    #[test]
    fn test_cache_max_age_parser() {
        let _ = parse_args(&["crates", "--cache-max-age", "7d"]).unwrap();
        let _ = parse_args(&["crates", "--cache-max-age=7d"]).unwrap();
        let _ = parse_args(&["crates", "--cache-max-age=1w"]).unwrap();
        let _ = parse_args(&["crates", "--cache-max-age=1m"]).unwrap();
        let _ = parse_args(&["crates", "--cache-max-age=1s"]).unwrap();
        // erroneous invocations that must be rejected
        assert!(parse_args(&["crates", "--cache-max-age"]).is_err());
        assert!(parse_args(&["crates", "--cache-max-age=5"]).is_err());
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
        let _ = parse_args(&["update", "--cache-max-age=7d"]).unwrap();
        // erroneous invocations that must be rejected
        assert!(parse_args(&["update", "-d"]).is_err());
        assert!(parse_args(&["update", "--diffable"]).is_err());
        assert!(parse_args(&["update", "-d", "--cache-max-age=7d"]).is_err());
        assert!(parse_args(&["update", "--diffable", "--cache-max-age=7d"]).is_err());
    }

    #[test]
    fn test_json_schema_option() {
        let _ = parse_args(&["json", "--print-schema"]).unwrap();
        // erroneous invocations that must be rejected
        assert!(parse_args(&["json", "--print-schema", "-d"]).is_err());
        assert!(parse_args(&["json", "--print-schema", "--diffable"]).is_err());
        assert!(parse_args(&["json", "--print-schema", "--cache-max-age=7d"]).is_err());
        assert!(
            parse_args(&["json", "--print-schema", "--diffable", "--cache-max-age=7d"]).is_err()
        );
    }

    #[test]
    fn test_invocation_through_cargo() {
        let _ = parse_args(&["supply-chain", "update"]).unwrap();
        let _ = parse_args(&["supply-chain", "publishers", "-d"]).unwrap();
        let _ = parse_args(&["supply-chain", "crates", "-d", "--cache-max-age=5h"]).unwrap();
        let _ = parse_args(&["supply-chain", "json", "--diffable"]).unwrap();
        let _ = parse_args(&["supply-chain", "json", "--print-schema"]).unwrap();
        // erroneous invocations to be rejected
        assert!(parse_args(&["supply-chain", "supply-chain", "json", "--print-schema"]).is_err());
        assert!(parse_args(&["supply-chain", "supply-chain", "crates", "-d"]).is_err());
    }
}
