//! Displays help information to the user when requested


use crate::err_exit;
use std::process;

const CLI_HELP: &'static str = "foobar"; //TODO

/// Provides help infomation which proceeds to exit
pub fn help(command: Option<&str>) {
    match command {
        None => println!("{}", CLI_HELP),
        Some("publishers") => println!("{}", PUBLISHERS_HELP),
        Some("crates") => println!("{}", CRATES_HELP),
        Some("update") => println!("{}", UPDATE_HELP),
        Some("json") => println!("{}\n{}", JSON_HELP, JSON_SCHEMA),
        Some(command) => {
            err_exit(format!("Unknown subcommand: {}\n{}", command, CLI_HELP).as_str())
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
  -d, --diffable  Make output more friendly towards tools such as `diff`

Any arguments after the `--` will be passed to `cargo metadata`, for example:
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.";

const PUBLISHERS_HELP: &str =
    "Lists all crates.io publishers in the dependency graph and owned crates for each

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
  -d, --diffable  Make output more friendly towards tools such as `diff`


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

USAGE:
  cargo supply-chain json [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

OPTIONS:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.
  -d, --diffable   Pretty-print the resulting JSON, making it easy to diff

Any arguments after the `--` will be passed to `cargo metadata`, for example:
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.

Note that detailed information on the origin of crates outside of crates.io is not
provided. You can obtain this info from 'cargo metadata' that ships with Cargo,
or use 'cargo deny' to define a custom policy regarding crate sources.

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

const JSON_SCHEMA: &str = r##"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "StructuredOutput",
  "type": "object",
  "required": [
    "crates_io_crates",
    "not_audited"
  ],
  "properties": {
    "crates_io_crates": {
      "description": "Maps crate names to info about the publishers of each crate",
      "type": "object",
      "additionalProperties": {
        "type": "array",
        "items": {
          "$ref": "#/definitions/PublisherData"
        }
      }
    },
    "not_audited": {
      "$ref": "#/definitions/NotAudited"
    }
  },
  "definitions": {
    "NotAudited": {
      "type": "object",
      "required": [
        "foreign_crates",
        "local_crates"
      ],
      "properties": {
        "foreign_crates": {
          "description": "Names of crates that are neither from crates.io nor from a local filesystem",
          "type": "array",
          "items": {
            "type": "string"
          }
        },
        "local_crates": {
          "description": "Names of crates that are imported from a location in the local filesystem, not from a registry",
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      }
    },
    "PublisherData": {
      "description": "Data about a single publisher received from a crates.io API endpoint",
      "type": "object",
      "required": [
        "id",
        "kind",
        "login"
      ],
      "properties": {
        "avatar": {
          "description": "Avatar image URL",
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "integer",
          "format": "uint64",
          "minimum": 0.0
        },
        "kind": {
          "$ref": "#/definitions/PublisherKind"
        },
        "login": {
          "type": "string"
        },
        "name": {
          "description": "Display name. It is NOT guaranteed to be unique!",
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "PublisherKind": {
      "type": "string",
      "enum": [
        "team",
        "user"
      ]
    }
  }
}"##;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subcommands::json::StructuredOutput;
    use schemars::schema_for;

    #[test]
    fn test_json_schema() {
        let schema = schema_for!(StructuredOutput);
        let schema = serde_json::to_string_pretty(&schema).unwrap();
        assert_eq!(schema, JSON_SCHEMA);
    }
}
