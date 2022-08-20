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
    JsonSchema,
    Update {
        cache_max_age: Duration,
    },
}

fn cache_max_age() -> impl Parser<Duration> {
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

fn args() -> impl Parser<QueryCommandArgs> {
    let diffable = short('d')
        .long("diffable")
        .help("Make output more friendly towards tools such as `diff`")
        .switch();
    construct!(QueryCommandArgs {
        cache_max_age(),
        diffable,
    })
}

fn meta_args() -> impl Parser<MetadataArgs> {
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
        let publishers_long = "\
If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.";
        let publishers_short =
            "Lists all crates.io publishers in the dependency graph and owned crates for each";
        let parser = construct!(CliArgs::Publishers { args(), meta_args() })
            .to_options()
            .descr(publishers_short)
            .header(publishers_long);

        command("publishers", parser)
    };

    let crates = {
        let parser = construct!(CliArgs::Crates { args(), meta_args() });
        let crates_long = "\
If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.";
        let crates_short = "List all crates in dependency graph and crates.io publishers for each";
        let parser = parser.to_options().descr(crates_short).header(crates_long);
        command("crates", parser)
    };

    let json = {
        let print_schema = long("print-schema")
            .help("Print JSON schema and exit")
            .req_flag(CliArgs::JsonSchema);
        let json = construct!(CliArgs::Json { args(), meta_args() });
        let parser = (construct!([json, print_schema])).to_options().descr(
            "\
Detailed info on publishers of all crates in the dependency graph, in JSON

The JSON schema is also available, use --print-schema to get it.

If a local cache created by 'update' subcommand is present and up to date,
it will be used. Otherwise live data will be fetched from the crates.io API.",
        );

        command("json", parser)
            .help("Like 'crates', but in JSON and with more fields for each publisher")
    };

    let update = construct!(CliArgs::Update { cache_max_age() })
        .to_options()
        .descr(
            "\
Download the latest daily dump from crates.io to speed up other commands

If the local cache is already younger than specified in '--cache-max-age' option,
a newer version will not be downloaded.

Note that this downloads the entire crates.io database, which is hundreds of Mb of data!
If you are on a metered connection, you should not be running the 'update' subcommand.
Instead, rely on requests to the live API - they are slower, but use much less data.",
        );

    let update = command("update", update)
        .help("Download the latest daily dump from crates.io to speed up other commands");

    cargo_helper(
        "supply-chain",
        construct!([publishers, crates, json, update]),
    )
    .to_options()
    .version(env!("CARGO_PKG_VERSION"))
    .descr("Gather author, contributor and publisher data on crates in your dependency graph")
    .footer(
        "\
Most commands also accept flags controlling the features, targets, etc.
See 'cargo supply-chain <command> --help' for more information on a specific command.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<CliArgs, ParseFailure> {
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
