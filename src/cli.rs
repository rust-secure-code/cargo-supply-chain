use bpaf::*;
use std::{path::PathBuf, time::Duration};

/// Arguments to be passed to `cargo metadata`
#[derive(Clone, Debug, Bpaf)]
#[bpaf(generate(meta_args))]
pub struct MetadataArgs {
    // `all_features` and `no_default_features` are not mutually exclusive in `cargo metadata`,
    // in the sense that it will not error out when encontering them; it just follows `all_features`
    /// Activate all available features
    pub all_features: bool,

    /// Do not activate the `default` feature
    pub no_default_features: bool,

    // This is a `String` because we don't parse the value, just pass it on to `cargo metadata` blindly
    /// Space or comma separated list of features to activate
    #[bpaf(argument("FEATURES"))]
    pub features: Option<String>,

    /// Only include dependencies matching the given target-triple
    #[bpaf(argument("TRIPLE"))]
    pub target: Option<String>,

    /// Path to Cargo.toml
    #[bpaf(argument("PATH"))]
    pub manifest_path: Option<PathBuf>,
}

/// Arguments for typical querying commands - crates, publishers, json
#[derive(Clone, Debug, Bpaf)]
#[bpaf(generate(args))]
pub(crate) struct QueryCommandArgs {
    #[bpaf(external)]
    pub cache_max_age: Duration,

    /// Make output more friendly towards tools such as `diff`
    #[bpaf(short, long)]
    pub diffable: bool,
}

#[derive(Clone, Debug, Bpaf)]
pub(crate) enum PrintJson {
    /// Print JSON schema and exit
    #[bpaf(long("print-schema"))]
    Schema,

    Info {
        #[bpaf(external)]
        args: QueryCommandArgs,
        #[bpaf(external)]
        meta_args: MetadataArgs,
    },
}

/// Gather author, contributor and publisher data on crates in your dependency graph
///
///
/// Most commands also accept flags controlling the features, targets, etc.
/// See 'cargo supply-chain <command> --help' for more information on a specific command.
#[derive(Clone, Debug, Bpaf)]
#[bpaf(options("supply-chain"), generate(args_parser), version)]
pub(crate) enum CliArgs {
    /// Lists all crates.io publishers in the dependency graph and owned crates for each
    ///
    ///
    /// If a local cache created by 'update' subcommand is present and up to date,
    /// it will be used. Otherwise live data will be fetched from the crates.io API.
    #[bpaf(command)]
    Publishers {
        #[bpaf(external)]
        args: QueryCommandArgs,
        #[bpaf(external)]
        meta_args: MetadataArgs,
    },

    /// List all crates in dependency graph and crates.io publishers for each
    ///
    ///
    /// If a local cache created by 'update' subcommand is present and up to date,
    /// it will be used. Otherwise live data will be fetched from the crates.io API.
    #[bpaf(command)]
    Crates {
        #[bpaf(external)]
        args: QueryCommandArgs,
        #[bpaf(external)]
        meta_args: MetadataArgs,
    },

    /// Detailed info on publishers of all crates in the dependency graph, in JSON
    ///
    /// The JSON schema is also available, use --print-schema to get it.
    ///
    /// If a local cache created by 'update' subcommand is present and up to date,
    /// it will be used. Otherwise live data will be fetched from the crates.io API.",
    #[bpaf(command)]
    Json(#[bpaf(external(print_json))] PrintJson),

    /// Download the latest daily dump from crates.io to speed up other commands
    ///
    ///
    /// If the local cache is already younger than specified in '--cache-max-age' option,
    /// a newer version will not be downloaded.
    ///
    /// Note that this downloads the entire crates.io database, which is hundreds of Mb of data!
    /// If you are on a metered connection, you should not be running the 'update' subcommand.
    /// Instead, rely on requests to the live API - they are slower, but use much less data.
    #[bpaf(command)]
    Update {
        #[bpaf(external)]
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
        .argument::<String>("AGE")
        .parse(|text| humantime::parse_duration(&text))
        .fallback(Duration::from_secs(48 * 3600))
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
