//! `json` subcommand is equivalent to `crates`,
//! but provides structured output and more info about each publisher.
use crate::publishers::{fetch_owners_of_crates, PublisherData};
use crate::{
    common::{crate_names_from_source, sourced_dependencies, PkgSource},
    MetadataArgs,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[cfg(test)]
use schemars::JsonSchema;

#[cfg_attr(test, derive(JsonSchema))]
#[derive(Debug, Serialize, Default, Clone)]
pub struct StructuredOutput {
    not_audited: NotAudited,
    /// Maps crate names to info about the publishers of each crate
    crates_io_crates: BTreeMap<String, Vec<PublisherData>>,
}

#[cfg_attr(test, derive(JsonSchema))]
#[derive(Debug, Serialize, Default, Clone)]
pub struct NotAudited {
    /// Names of crates that are imported from a location in the local filesystem, not from a registry
    local_crates: Vec<String>,
    /// Names of crates that are neither from crates.io nor from a local filesystem
    foreign_crates: Vec<String>,
}

pub fn json(
    args: MetadataArgs,
    diffable: bool,
    max_age: std::time::Duration,
) -> Result<(), anyhow::Error> {
    let mut output = StructuredOutput::default();
    let dependencies = sourced_dependencies(args)?;
    // Report non-crates.io dependencies
    output.not_audited.local_crates = crate_names_from_source(&dependencies, PkgSource::Local);
    output.not_audited.foreign_crates = crate_names_from_source(&dependencies, PkgSource::Foreign);
    output.not_audited.local_crates.sort_unstable();
    output.not_audited.foreign_crates.sort_unstable();
    // Fetch list of owners and publishers
    let (mut owners, publisher_teams) = fetch_owners_of_crates(&dependencies, max_age)?;
    // Merge the two maps we received into one
    for (crate_name, publishers) in publisher_teams {
        owners.entry(crate_name).or_default().extend(publishers);
    }
    // Sort the vectors of publisher data. This helps when diffing the output,
    // but we do it unconditionally because it's cheap and helps users pull less hair when debugging.
    for list in owners.values_mut() {
        list.sort_unstable_by_key(|x| x.id);
    }
    output.crates_io_crates = owners;
    // Print the result to stdout
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    if diffable {
        let value = serde_json::to_value(&output)?;
        serde_json::to_writer_pretty(handle, &value)?;
    } else {
        serde_json::to_writer(handle, &output)?;
    }
    Ok(())
}
