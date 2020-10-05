//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.
use cargo_metadata::{CargoOpt::AllFeatures, MetadataCommand, Package, PackageId};
use common::*;
use publishers::PublisherData;
use std::collections::{BTreeMap, HashMap, HashSet};

mod authors;
mod common;
mod crates_io;
mod publishers;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.by_ref().next();

    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("authors") => return authors(args),
            Some("publishers") => return publishers(args),
            Some(arg) if arg.starts_with("--") => bail_unknown_option(arg),
            Some(arg) if arg.starts_with('-') => bail_unknown_short_option(arg),
            Some(arg) => bail_unknown_command(arg),
        }
    }

    // No tool selected.
    bail_no_tool();
}

fn authors(mut args: std::env::ArgsOs) {
    if let Some(arg) = args.next() {
        bail_unknown_author_arg(arg)
    }

    let dependencies = sourced_dependencies();

    let authors: HashSet<_> = authors::authors_of(&dependencies).collect();
    let mut display_authors: Vec<_> = authors.iter().map(|a| a.to_string()).collect();
    display_authors.sort_unstable();
    for a in display_authors {
        println!("{}", a);
    }
}

fn publishers(mut args: std::env::ArgsOs) {
    if let Some(arg) = args.next() {
        bail_unknown_publishers_arg(arg)
    }
    let dependencies = sourced_dependencies();

    let local_crate_names = crate_names_from_source(&dependencies, PkgSource::Local);
    if local_crate_names.len() > 0 {
        println!("\nThe following crates will be ignored because they come from a local directory:");
        for crate_name in &local_crate_names {
            println!(" - {}", crate_name);
        }
    }

    let foreign_crate_names = crate_names_from_source(&dependencies, PkgSource::Foreign);
    if local_crate_names.len() > 0 {
        println!("\nCannot audit the following crates because they are not from crates.io:");
        for crate_name in &foreign_crate_names {
            println!(" - {}", crate_name);
        }
    }

    let crates_io_names = crate_names_from_source(&dependencies, PkgSource::CratesIo);
    let mut client = crates_io::ApiClient::new();
    let mut publisher_users: HashMap<String, Vec<PublisherData>> = HashMap::new();
    let mut publisher_teams: HashMap<String, Vec<PublisherData>> = HashMap::new();
    eprintln!("\nFetching publisher info from crates.io");
    eprintln!("This will take roughly 2 seconds per crate due to API rate limits");
    for (i, crate_name) in crates_io_names.iter().enumerate() {
        eprintln!(
            "Fetching data for \"{}\" ({}/{})",
            crate_name,
            i,
            crates_io_names.len()
        );
        publisher_users.insert(
            crate_name.clone(),
            publishers::publisher_users(&mut client, crate_name).unwrap(),
        );
        publisher_teams.insert(
            crate_name.clone(),
            publishers::publisher_teams(&mut client, crate_name).unwrap(),
        );
    }

    if publisher_users.len() > 0 {
        println!("\nThe following individuals can publish updates for your dependencies:\n");
        let user_to_crate_map = transpose_publishers_map(&publisher_users);
        let map_for_display = sort_transposed_map_for_display(user_to_crate_map);
        for (i, (user, crates)) in map_for_display.iter().enumerate() {
            // We do not print usernames, since you can embed terminal control sequences in them
            // and erase yourself from the output that way.
            // TODO: check if it's possible to smuggle those into github/crates.io usernames
            let crate_list = comma_separated_list(&crates);
            println!(" {}. {} via crates: {}", i+1, &user.login, crate_list);
        }
    }

    println!("\nNote: there may be outstanding publisher invitations. crates.io provides no way to list them.");
    println!("Invitations are also impossible to revoke, and they never expire.");
    println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");

    if publisher_teams.len() > 0 {
        println!("\nAll members of the following teams can publish updates for your dependencies:\n");
        let team_to_crate_map = transpose_publishers_map(&publisher_teams);
        let map_for_display = sort_transposed_map_for_display(team_to_crate_map);
        for (i, (team, crates)) in map_for_display.iter().enumerate() {
            let crate_list = comma_separated_list(&crates);
            if let Some(url) = &team.url {
                println!(
                    " {}. \"{}\" ({}) via crates: {}",
                    i+1, &team.login, url, crate_list
                );
            } else {
                println!(" {}. \"{}\" via crates: {}", i+1, &team.login, crate_list);
            }
        }
        println!("\nGithub teams are black boxes. It's impossible to get the member list without explicit permission.");
    }
}

fn comma_separated_list(list: &[String]) -> String {
    let mut result = String::new();
    let mut first_loop = true;
    for crate_name in list {
        if !first_loop {
            result.push_str(", ");
        }
        first_loop = false;
        result.push_str(crate_name.as_str());
    }
    result
}

fn crate_names_from_source(crates: &[SourcedPackage], source: PkgSource) -> Vec<String> {
    let mut filtered_crate_names: Vec<String> = crates
        .iter()
        .filter(|p| p.source == source)
        .map(|p| p.package.name.clone())
        .collect();
    // Collecting into a HashSet is less user-friendly because order varies between runs
    filtered_crate_names.sort_unstable();
    filtered_crate_names.dedup();
    filtered_crate_names
}

/// Turns a crate-to-publishers mapping into publisher-to-crates mapping.
/// BTreeMap is used because PublisherData doesn't implement Hash.
fn transpose_publishers_map(
    input: &HashMap<String, Vec<PublisherData>>,
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
fn sort_transposed_map_for_display(input: BTreeMap<PublisherData, Vec<String>>) -> Vec<(PublisherData, Vec<String>)> {
    let mut result: Vec<_> = input.into_iter().collect();
    result.sort_unstable_by_key(|(publisher, crates)| {
        (usize::MAX - crates.len(), publisher.login.clone())
    });
    result
}

fn sourced_dependencies() -> Vec<SourcedPackage> {
    let meta = MetadataCommand::new().features(AllFeatures).exec().unwrap();

    let mut how: HashMap<PackageId, PkgSource> = HashMap::new();
    let what: HashMap<PackageId, Package> = meta
        .packages
        .iter()
        .map(|package| (package.id.clone(), package.clone()))
        .collect();

    for pkg in &meta.packages {
        // Suppose every package is foreign, until proven otherwise..
        how.insert(pkg.id.clone(), PkgSource::Foreign);
    }

    // Find the crates.io dependencies..
    for pkg in meta.packages {
        for dep in &pkg.dependencies {
            if let Some(source) = dep.source.as_ref() {
                if source == "registry+https://github.com/rust-lang/crates.io-index" {
                    how.insert(pkg.id.clone(), PkgSource::CratesIo);
                }
            }
        }
    }

    for pkg in meta.workspace_members {
        *how.get_mut(&pkg).unwrap() = PkgSource::Local;
    }

    let dependencies: Vec<_> = how
        .iter()
        .map(|(id, kind)| {
            let dep = what.get(id).cloned().unwrap();
            SourcedPackage {
                source: kind.clone(),
                package: dep,
            }
        })
        .collect();

    dependencies
}

fn bail_unknown_option(arg: &str) -> ! {
    eprintln!("Unknown option: {}", std::path::Path::new(&arg).display());
    std::process::exit(1);
}

fn bail_unknown_short_option(arg: &str) -> ! {
    eprintln!("Unknown flag: {}", arg);
    std::process::exit(1);
}

fn bail_unknown_command(arg: &str) -> ! {
    eprintln!("Unknown command: {}", arg);
    std::process::exit(1);
}

fn bail_unknown_author_arg(arg: std::ffi::OsString) {
    eprintln!(
        "Bad argument to authors command: {}",
        std::path::Path::new(&arg).display()
    );
    std::process::exit(1);
}

fn bail_unknown_publishers_arg(arg: std::ffi::OsString) {
    eprintln!(
        "Bad argument to publishers command: {}",
        std::path::Path::new(&arg).display()
    );
    std::process::exit(1);
}

fn bail_bad_arg(arg: std::ffi::OsString) -> ! {
    eprintln!("Bad argument: {}", std::path::Path::new(&arg).display());
    std::process::exit(1);
}

fn bail_no_tool() -> ! {
    eprintln!("No tool selected.");
    eprint_help();
    std::process::exit(1);
}

fn eprint_help() {
    eprintln!(
        "Usage: cargo supply-chain COMMAND [OPTIONS...]\n

  Commands:
    authors\t\tList all authors in the dependency graph\n
    publishers\t\tList all publishers in the dependency graph\n
"
    );
}
