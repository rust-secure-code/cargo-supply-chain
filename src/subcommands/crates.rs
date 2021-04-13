use crate::common::*;
use crate::publishers::{fetch_owners_of_crates, PublisherKind};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct CrateData {
    users: Vec<String>,
    teams: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct CratesJdoc {
    warning: String,
    description: String,
    complaint: String,
    crates: HashMap<String, CrateData>,
}

pub fn crates(
    args: Vec<String>,
    max_age: std::time::Duration,
    output_json: bool,
) -> Result<(), std::io::Error> {
    let dependencies = sourced_dependencies(args);
    let complaint = complain_about_non_crates_io_crates(&dependencies);
    let (mut owners, publisher_teams) = fetch_owners_of_crates(&dependencies, max_age)?;

    for (crate_name, publishers) in publisher_teams {
        owners.entry(crate_name).or_default().extend(publishers)
    }

    let mut ordered_owners: Vec<_> = owners.into_iter().collect();
    // Put crates owned by teams first
    ordered_owners.sort_unstable_by_key(|(name, publishers)| {
        (
            publishers
                .iter()
                .find(|p| p.kind == PublisherKind::team)
                .is_none(), // contains at least one team
            usize::MAX - publishers.len(),
            name.clone(),
        )
    });
    for (_, publishers) in ordered_owners.iter_mut() {
        // For each crate put teams first
        publishers.sort_unstable_by_key(|p| (p.kind, p.login.clone()));
    }

    if output_json {
        let mut jdoc = CratesJdoc{
            warning: concat! (
                "Note: there may be outstanding publisher invitations. crates.io provides no way to list them. ",
                "See https://github.com/rust-lang/crates.io/issues/2868 for more info."
            ).to_string(),
            description: concat! ("Dependency crates with the people and teams that ",
                "can publish them to crates.io"
            ).to_string(),
            complaint: complaint,
            crates: HashMap::new()
        };
        for (i, (crate_name, publishers)) in ordered_owners.iter().enumerate() {
            let user_publishers: Vec<String> = publishers
                .iter()
                .map(|p| match p.kind {
                    PublisherKind::team => "".to_string(),
                    PublisherKind::user => p.login.to_string(),
                })
                .filter(|x| !x.is_empty())
                .collect();
            let team_publishers: Vec<String> = publishers
                .iter()
                .map(|p| match p.kind {
                    PublisherKind::team => p.login.to_string(),
                    PublisherKind::user => "".to_string(),
                })
                .filter(|x| !x.is_empty())
                .collect();
            if jdoc.crates.contains_key(&crate_name.to_string()) {
                panic!(
                    "Collision detected in crates.io names for crate {}. Results unreliable.",
                    crate_name
                );
            } else {
                jdoc.crates.insert(
                    crate_name.to_string(),
                    CrateData {
                        users: user_publishers,
                        teams: team_publishers,
                    },
                );
            }
        }
        // output the document
        println!("{}", serde_json::to_string_pretty(&jdoc)?);
    } else {
        println!(
            "\nDependency crates with the people and teams that can publish them to crates.io:\n"
        );
        for (i, (crate_name, publishers)) in ordered_owners.iter().enumerate() {
            let pretty_publishers: Vec<String> = publishers
                .iter()
                .map(|p| match p.kind {
                    PublisherKind::team => format!("team \"{}\"", p.login),
                    PublisherKind::user => p.login.to_string(),
                })
                .collect();
            let publishers_list = comma_separated_list(&pretty_publishers);
            println!("{}. {}: {}", i + 1, crate_name, publishers_list);
        }

        if !ordered_owners.is_empty() {
            println!("\nNote: there may be outstanding publisher invitations. crates.io provides no way to list them.");
            println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");
        }
    }
    Ok(())
}
