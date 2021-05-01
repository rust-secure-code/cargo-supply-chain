//! `json` subcommand is equivalent to `crates`,
//! but provides structured output and more info about each publisher.
use crate::common::*;
use crate::publishers::{fetch_owners_of_crates, PublisherData};
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(JsonSchema, Debug, Serialize, Default, Clone)]
pub struct StructuredOutput {
    not_audited: NotAudited,
    /// Maps crate names to info about the publishers of each crate
    crates_io_crates: BTreeMap<String, Vec<PublisherData>>,
}

#[derive(JsonSchema, Debug, Serialize, Default, Clone)]
pub struct NotAudited {
    /// Names of crates that are imported from a location in the local filesystem, not from a registry
    local_crates: Vec<String>,
    /// Names of crates that are neither from crates.io nor from a local filesystem
    foreign_crates: Vec<String>,
}

pub fn json(args: Vec<String>, max_age: std::time::Duration) -> Result<(), std::io::Error> {
    let mut output = StructuredOutput::default();
    let dependencies = sourced_dependencies(args);
    // Report non-crates.io dependencies
    output.not_audited.local_crates = crate_names_from_source(&dependencies, PkgSource::Local);
    output.not_audited.foreign_crates = crate_names_from_source(&dependencies, PkgSource::Foreign);
    // Fetch list of owners and publishers
    let (mut owners, mut publisher_teams) = fetch_owners_of_crates(&dependencies, max_age)?;
    // Sort the vectors of publisher data so that users could diff the output.
    // That's not a super common thing to do, but sorting is cheap, so why not.
    for list in owners.values_mut().chain(publisher_teams.values_mut()) {
        list.sort_unstable_by_key(|x| x.login.clone())
    }
    // Merge the two maps we received into one
    for (crate_name, publishers) in publisher_teams {
        owners.entry(crate_name).or_default().extend(publishers)
    }
    output.crates_io_crates = owners;
    // Print the result to stdout
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    serde_json::to_writer(handle, &output)?;
    Ok(())
}