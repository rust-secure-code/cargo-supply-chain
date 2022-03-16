//! The schema for the JSON subcommand output

use std::io::{Result, Write};

pub fn print_schema() -> Result<()> {
    writeln!(std::io::stdout(), "{}", JSON_SCHEMA)?;
    Ok(())
}

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
