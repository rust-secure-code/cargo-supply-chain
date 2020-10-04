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
use std::collections::{HashMap, HashSet};

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

    let authors: HashSet<_> = authors::authors_of(&dependencies).collect();
    let mut display_authors: Vec<_> = authors.iter().map(|a| a.to_string()).collect();
    display_authors.sort_unstable();
    for a in display_authors {
        println!("{}", a);
    }
}

fn owners(mut args: std::env::ArgsOs) {
    if let Some(arg) = args.next() {
        bail_unknown_author_arg(arg)
    }
    let mut client = crates_io::ApiClient::new();
    let result = owners::owner_users(&mut client, "auditable").unwrap();
    println!("{:?}", result);
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
