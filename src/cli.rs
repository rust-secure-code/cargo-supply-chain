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

pub(crate) fn args_parser() -> OptionParser<CliArgs> {
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
            "json" => {
                let print_schema = long("print-schema")
                    .help("Print JSON schema and exit")
                    .req_flag(());
                construct!(CliArgs::Json { args, meta_args })
                    .or_else(construct!(CliArgs::JsonSchema { print_schema }))
            }
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

The JSON schema is also available, use --print-schema to get it.

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
        .footer(
            "\
Most commands also accept flags controlling the features, targets, etc.
See 'cargo supply-chain <command> --help' for more information on a specific command.",
        )
        .for_parser(parser)
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

    #[test]
    fn test_json_schema_option() {
        let _ = args_parser()
            .run_inner(Args::from(&["json", "--print-schema"]))
            .unwrap();
        // erroneous invocations that must be rejected
        assert!(args_parser()
            .run_inner(Args::from(&["json", "--print-schema", "-d"]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&["json", "--print-schema", "--diffable"]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&[
                "json",
                "--print-schema",
                "--cache-max-age=7d"
            ]))
            .is_err());
        assert!(args_parser()
            .run_inner(Args::from(&[
                "json",
                "--print-schema",
                "--diffable",
                "--cache-max-age=7d"
            ]))
            .is_err());
    }
}
