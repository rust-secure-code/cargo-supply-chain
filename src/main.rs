//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use common::*;
use publishers::{fetch_owners_of_crates, PublisherData, PublisherKind};
use std::collections::HashMap;

mod api_client;
mod authors;
mod common;
mod publishers;
mod subcommands;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.by_ref().next();

    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("supply-chain") => (), // first arg when run as `cargo supply-chain`
            Some("authors") => return subcommands::authors(args),
            Some("publishers") => return subcommands::publishers(args),
            Some("crates") => return crates(args),
            Some(arg) if arg.starts_with("--") => bail_unknown_option(arg),
            Some(arg) if arg.starts_with('-') => bail_unknown_short_option(arg),
            Some(arg) => bail_unknown_command(arg),
        }
    }

    // No tool selected.
    bail_no_tool();
}

fn crates(mut args: std::env::ArgsOs) {
    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("--") => break, // we pass args after this to cargo-metadata
            _ => bail_unknown_subcommand_arg("crates", arg),
        }
    }

    let dependencies = sourced_dependencies(args);
    complain_about_non_crates_io_crates(&dependencies);
    let (publisher_users, publisher_teams) = fetch_owners_of_crates(&dependencies);

    // Merge maps back together. Ewww. Maybe there's a better way to go about this.
    let mut owners: HashMap<String, Vec<PublisherData>> = HashMap::new();
    for map in &[publisher_users, publisher_teams] {
        for (crate_name, publishers) in map.iter() {
            let entry = owners.entry(crate_name.clone()).or_default();
            entry.extend_from_slice(publishers);
        }
    }

    let mut ordered_owners: Vec<_> = owners.into_iter().collect();
    // Put crates owned by teams first
    ordered_owners.sort_unstable_by_key(|(name, publishers)| {
        (
            publishers
                .iter()
                .filter(|p| p.kind == PublisherKind::team)
                .next()
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
    for (i, (crate_name, publishers)) in ordered_owners.iter().enumerate() {
        let pretty_publishers: Vec<String> = publishers
            .iter()
            .map(|p| match p.kind {
                PublisherKind::team => format!("team \"{}\"", p.login),
                PublisherKind::user => format!("{}", p.login),
            })
            .collect();
        let publishers_list = comma_separated_list(&pretty_publishers);
        println!("{}. {}: {}", i + 1, crate_name, publishers_list);
    }

    if ordered_owners.len() > 0 {
        println!("\nNote: there may be outstanding publisher invitations. crates.io provides no way to list them.");
        println!("Invitations are also impossible to revoke, and they never expire.");
        println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");
    }
}

fn bail_unknown_option(arg: &str) -> ! {
    eprintln!("Unknown option: {}", std::path::Path::new(&arg).display());
    eprint_help();
    std::process::exit(1);
}

fn bail_unknown_short_option(arg: &str) -> ! {
    eprintln!("Unknown flag: {}", arg);
    eprint_help();
    std::process::exit(1);
}

fn bail_unknown_command(arg: &str) -> ! {
    eprintln!("Unknown command: {}", arg);
    eprint_help();
    std::process::exit(1);
}

fn bail_no_tool() -> ! {
    eprintln!("No tool selected.");
    eprint_help();
    std::process::exit(1);
}

fn eprint_help() {
    eprintln!(
        "Usage: cargo supply-chain COMMAND [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

  Commands:
    authors\t\tList all authors in the dependency graph (as specified in Cargo.toml)
    publishers\t\tList all crates.io publishers in the dependency graph
    crates\t\tList all crates in dependency graph and crates.io publishers for each

  Any arguments after -- will be passed to `cargo metadata`, for example:
    cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
"
    );
}
