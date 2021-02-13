use std::collections::HashMap;

use crate::common::*;
use crate::publishers::{fetch_owners_of_crates, CrateOwners, PublisherData, PublisherKind};

pub fn crates(args: Vec<String>, max_age: std::time::Duration) -> Result<(), std::io::Error> {
    let dependencies = sourced_dependencies(args);
    complain_about_non_crates_io_crates(&dependencies);
    let CrateOwners { users, teams } = fetch_owners_of_crates(&dependencies, max_age)?;

    let owners: HashMap<String, Vec<PublisherData>> =
        users
            .into_iter()
            .chain(teams)
            .fold(HashMap::new(), |mut owners, package| {
                let (crate_name, mut publishers) = package;
                let entry = owners.entry(crate_name).or_default();

                entry.append(&mut publishers);
                owners
            });

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
    for (_name, publishers) in ordered_owners.iter_mut() {
        // For each crate put teams first
        publishers.sort_unstable_by_key(|p| (p.kind, p.login.clone()));
    }

    println!("\nDependency crates with the people and teams that can publish them to crates.io:\n");

    let show_outstanding_warning = !ordered_owners.is_empty();
    for (i, (crate_name, publishers)) in ordered_owners.into_iter().enumerate() {
        let pretty_publishers: Vec<String> = publishers
            .into_iter()
            .map(|p| match p.kind {
                PublisherKind::team => format!("team \"{}\"", p.login),
                PublisherKind::user => p.login,
            })
            .collect();

        let publishers_list = comma_separated_list(pretty_publishers.iter().map(|s| s.as_str()));
        println!("{}. {}: {}", i + 1, crate_name, publishers_list);
    }

    if show_outstanding_warning {
        println!("\nNote: there may be outstanding publisher invitations. crates.io provides no way to list them.");
        println!("Invitations are also impossible to revoke, and they never expire.");
        println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");
    }
    Ok(())
}
