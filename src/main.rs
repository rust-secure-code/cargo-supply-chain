//! Gather author, contributor, owner data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.
use cargo_metadata::{CargoOpt::AllFeatures, MetadataCommand, Package, PackageId};
use common::*;
use owners::OwnerData;
use std::collections::{BTreeMap, HashMap, HashSet};

mod authors;
mod common;
mod crates_io;
mod owners;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.by_ref().next();

    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("authors") => return authors(args),
            Some("owners") => return owners(args),
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

fn owners(mut args: std::env::ArgsOs) {
    if let Some(arg) = args.next() {
        bail_unknown_owners_arg(arg)
    }
    let dependencies = sourced_dependencies();
    let mut crates_io_names: Vec<String> = dependencies
        .iter()
        .filter(|p| p.source == PkgSource::CratesIo)
        .map(|p| p.package.name.clone())
        .collect();
    // Collecting into a HashSet is less user-friendly because order varies between runs
    crates_io_names.sort_unstable();
    crates_io_names.dedup();

    // TODO: list local crates

    // TODO: list crates from git or registiers other than crates.io

    eprintln!("Fetching owner info from crates.io");
    eprintln!("This will take roughly 2 seconds per crate due to API rate limits");
    let mut client = crates_io::ApiClient::new();
    let mut owner_users: HashMap<String, Vec<OwnerData>> = HashMap::new();
    let mut owner_teams: HashMap<String, Vec<OwnerData>> = HashMap::new();
    for (i, crate_name) in crates_io_names.iter().enumerate() {
        eprintln!(
            "Fetching data for \"{}\" ({}/{})",
            crate_name,
            i,
            crates_io_names.len()
        );
        owner_users.insert(
            crate_name.clone(),
            owners::owner_users(&mut client, crate_name).unwrap(),
        );
        owner_teams.insert(
            crate_name.clone(),
            owners::owner_teams(&mut client, crate_name).unwrap(),
        );
    }

    // TODO: list individual owners
    
    println!("\nNote: there may be outstanding owner invitations. crates.io provides no way to list them.");
    println!("Invitations are also impossible to revoke, and they never expire.");
    println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");

    if owner_teams.len() > 0 {
        println!("\nYou also implicitly trust all members of the following teams:\n");
        let team_to_crate_map = transpose_owners_map(&owner_teams);
        for (team, crates) in team_to_crate_map.iter() {
            let crate_list = pretty_print_crate_list(&crates);
            if let Some(url) = &team.url {
                println!(" - \"{}\" ({}) via crates: {}", &team.login, url, crate_list);
            } else {
                println!(" - \"{}\" via crates: {}", &team.login, crate_list);
            }
        }
        println!("\nGithub teams are black boxes. It's impossible to get the member list without explicit permission.");
    }
}

fn pretty_print_crate_list(list: &[String]) -> String {
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

/// Turns a crate-to-owners mapping into owner-to-crates mapping.
/// BTreeMap is used because OwnerData doesn't implement Hash.
fn transpose_owners_map(
    input: &HashMap<String, Vec<OwnerData>>,
) -> BTreeMap<OwnerData, Vec<String>> {
    let mut result: BTreeMap<OwnerData, Vec<String>> = BTreeMap::new();
    for (crate_name, owners) in input.iter() {
        for owner in owners {
            result
                .entry(owner.clone())
                .or_default()
                .push(crate_name.clone());
        }
    }
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

fn bail_unknown_owners_arg(arg: std::ffi::OsString) {
    eprintln!(
        "Bad argument to owners command: {}",
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
    owners\t\tList all owners in the dependency graph\n
"
    );
}
