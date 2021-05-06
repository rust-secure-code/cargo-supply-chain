use std::collections::BTreeMap;

use crate::publishers::fetch_owners_of_crates;
use crate::{common::*, publishers::PublisherData};

pub fn publishers(
    metadata_args: Vec<String>,
    diffable: bool,
    max_age: std::time::Duration,
) -> Result<(), std::io::Error> {
    let dependencies = sourced_dependencies(metadata_args);
    complain_about_non_crates_io_crates(&dependencies);
    let (publisher_users, publisher_teams) = fetch_owners_of_crates(&dependencies, max_age)?;

    // Group data by user rather than by crate
    let mut user_to_crate_map = transpose_publishers_map(&publisher_users);
    let mut team_to_crate_map = transpose_publishers_map(&publisher_teams);

    // Sort crate names alphabetically
    user_to_crate_map.values_mut().for_each(|c| c.sort());
    team_to_crate_map.values_mut().for_each(|c| c.sort());

    if !publisher_users.is_empty() && !diffable {
        println!("\nThe following individuals can publish updates for your dependencies:\n");
        let map_for_display = sort_transposed_map_for_display(user_to_crate_map);
        for (i, (user, crates)) in map_for_display.iter().enumerate() {
            // We do not print usernames, since you can embed terminal control sequences in them
            // and erase yourself from the output that way.
            let crate_list = comma_separated_list(&crates);
            println!(" {}. {} via crates: {}", i + 1, &user.login, crate_list);
        }
        println!("\nNote: there may be outstanding publisher invitations. crates.io provides no way to list them.");
        println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");
    } else if diffable {
        // empty map just means 0 loop iterations here
        let sorted_map = sort_transposed_map_for_diffing(user_to_crate_map);
        for (user, crates) in sorted_map.iter() {
            let crate_list = comma_separated_list(&crates);
            println!("user \"{}\": {}", &user.login, crate_list);
        }
    }

    if !publisher_teams.is_empty() && !diffable {
        println!(
            "\nAll members of the following teams can publish updates for your dependencies:\n"
        );
        let map_for_display = sort_transposed_map_for_display(team_to_crate_map);
        for (i, (team, crates)) in map_for_display.iter().enumerate() {
            let crate_list = comma_separated_list(&crates);
            if let (true, Some(org)) = (
                team.login.starts_with("github:"),
                team.login.split(':').nth(1),
            ) {
                println!(
                    " {}. \"{}\" (https://github.com/{}) via crates: {}",
                    i + 1,
                    &team.login,
                    org,
                    crate_list
                );
            } else {
                println!(" {}. \"{}\" via crates: {}", i + 1, &team.login, crate_list);
            }
        }
        println!("\nGithub teams are black boxes. It's impossible to get the member list without explicit permission.");
    } else if diffable {
        let sorted_map = sort_transposed_map_for_diffing(team_to_crate_map);
        for (team, crates) in sorted_map.iter() {
            let crate_list = comma_separated_list(&crates);
            println!("team \"{}\": {}", &team.login, crate_list);
        }
    }
    Ok(())
}

/// Turns a crate-to-publishers mapping into publisher-to-crates mapping.
/// BTreeMap is used because PublisherData doesn't implement Hash.
fn transpose_publishers_map(
    input: &BTreeMap<String, Vec<PublisherData>>,
) -> BTreeMap<PublisherData, Vec<String>> {
    let mut result: BTreeMap<PublisherData, Vec<String>> = BTreeMap::new();
    for (crate_name, publishers) in input.iter() {
        for publisher in publishers {
            result
                .entry(publisher.clone())
                .or_default()
                .push(crate_name.clone());
        }
    }
    result
}

/// Returns a Vec sorted so that publishers are sorted by the number of crates they control.
/// If that number is the same, sort by login.
fn sort_transposed_map_for_display(
    input: BTreeMap<PublisherData, Vec<String>>,
) -> Vec<(PublisherData, Vec<String>)> {
    let mut result: Vec<_> = input.into_iter().collect();
    result.sort_unstable_by_key(|(publisher, crates)| {
        (usize::MAX - crates.len(), publisher.login.clone())
    });
    result
}

fn sort_transposed_map_for_diffing(
    input: BTreeMap<PublisherData, Vec<String>>,
) -> Vec<(PublisherData, Vec<String>)> {
    let mut result: Vec<_> = input.into_iter().collect();
    result.sort_unstable_by_key(|(publisher, _crates)| publisher.login.clone());
    result
}
