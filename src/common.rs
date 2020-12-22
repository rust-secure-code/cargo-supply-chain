use cargo_metadata::{CargoOpt::AllFeatures, MetadataCommand, Package, PackageId};
use std::collections::HashMap;
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PkgSource {
    Local,
    CratesIo,
    Foreign,
}
#[derive(Debug, Clone)]
pub struct SourcedPackage {
    pub source: PkgSource,
    pub package: Package,
}

pub fn sourced_dependencies(extra_options: Vec<String>) -> Vec<SourcedPackage> {
    let meta = MetadataCommand::new()
        .features(AllFeatures)
        .other_options(extra_options)
        .exec()
        .unwrap();

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
    for pkg in &meta.packages {
        if let Some(source) = pkg.source.as_ref() {
            if source.is_crates_io() {
                how.insert(pkg.id.clone(), PkgSource::CratesIo);
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

pub fn crate_names_from_source(crates: &[SourcedPackage], source: PkgSource) -> Vec<String> {
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

pub fn complain_about_non_crates_io_crates(dependencies: &[SourcedPackage]) {
    {
        // scope bound to avoid accidentally referencing local crates when working with foreign ones
        let local_crate_names = crate_names_from_source(dependencies, PkgSource::Local);
        if local_crate_names.len() > 0 {
            println!(
                "\nThe following crates will be ignored because they come from a local directory:"
            );
            for crate_name in &local_crate_names {
                println!(" - {}", crate_name);
            }
        }
    }

    {
        let foreign_crate_names = crate_names_from_source(dependencies, PkgSource::Foreign);
        if foreign_crate_names.len() > 0 {
            println!("\nCannot audit the following crates because they are not from crates.io:");
            for crate_name in &foreign_crate_names {
                println!(" - {}", crate_name);
            }
        }
    }
}

pub fn comma_separated_list(list: &[String]) -> String {
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

pub fn get_argument<T, E: std::fmt::Display>(
    what: std::ffi::OsString,
    args: &mut std::env::ArgsOs,
    parser: impl FnOnce(&str) -> Result<T, E>,
) -> T {
    let arg = match args.next() {
        Some(arg) => arg,
        None => bail_missing_argument(what),
    };
    let arg_str = match arg.to_str() {
        Some(arg) => arg,
        None => bail_invalid_argument(what, std::path::Path::new(&arg).display()),
    };
    parser(arg_str).unwrap_or_else(|err| bail_invalid_argument(what, err))
}

pub fn bail_bad_arg(arg: std::ffi::OsString) -> ! {
    eprintln!("Bad argument: {}", std::path::Path::new(&arg).display());
    std::process::exit(1);
}

pub fn bail_missing_argument(arg: std::ffi::OsString) -> ! {
    eprintln!(
        "Missing argument to {}",
        std::path::Path::new(&arg).display()
    );
    std::process::exit(1);
}

pub fn bail_invalid_argument(arg: std::ffi::OsString, err: impl std::fmt::Display) -> ! {
    eprintln!(
        "Invalid argument to {}: {}",
        std::path::Path::new(&arg).display(),
        err
    );
    std::process::exit(1);
}

pub fn bail_unknown_subcommand_arg(subcommand: &str, arg: std::ffi::OsString) {
    eprintln!(
        "Bad argument to {} command: {}",
        subcommand,
        std::path::Path::new(&arg).display()
    );
    std::process::exit(1);
}
