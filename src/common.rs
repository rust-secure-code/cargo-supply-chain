use crate::{err_exit};
use cargo_metadata::{
    CargoOpt::AllFeatures, CargoOpt::NoDefaultFeatures, MetadataCommand, Package, PackageId,
};
use std::{collections::HashMap, path::PathBuf};

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

/// Arguments to be passed to `cargo metadata`
#[derive(Clone, Debug)]
pub struct MetadataArgs {
    // `all_features` and `no_default_features` are not mutually exclusive in `cargo metadata`,
    // in the sense that it will not error out when encontering them; it just follows `all_features`
    pub all_features: bool,
    pub no_default_features: bool,
    // This is a `String` because we don't parse the value, just pass it on to `cargo metadata` blindly
    pub features: Option<String>,
    pub target: Option<String>,
    pub manifest_path: Option<PathBuf>,
}

fn metadata_command(args: MetadataArgs) -> MetadataCommand {
    let mut command = MetadataCommand::new();
    if args.all_features {
        command.features(AllFeatures);
    }
    if args.no_default_features {
        command.features(NoDefaultFeatures);
    }
    if let Some(path) = args.manifest_path {
        command.manifest_path(path);
    }
    if let Some(target) = args.target {
        command.manifest_path(target);
    }
    // `cargo-metadata` crate assumes we have a Vec of features,
    // but we really didn't want to parse it ourselves, so we pass the argument directly
    if let Some(features) = args.features {
        command.other_options(vec![format!("--target={}", features)]);
    }
    command
}

pub fn sourced_dependencies(metadata_args: MetadataArgs) -> Vec<SourcedPackage> {
    let command = metadata_command(metadata_args);
    let meta = match command.exec() {
        Ok(v) => v,
        Err(cargo_metadata::Error::CargoMetadata { stderr: e }) => err_exit(&e),
        Err(err) => err_exit(format!("Failed to fetch crate metadata!\n  {}", err).as_str()),
    };

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
        if !local_crate_names.is_empty() {
            eprintln!(
                "\nThe following crates will be ignored because they come from a local directory:"
            );
            for crate_name in &local_crate_names {
                eprintln!(" - {}", crate_name);
            }
        }
    }

    {
        let foreign_crate_names = crate_names_from_source(dependencies, PkgSource::Foreign);
        if !foreign_crate_names.is_empty() {
            eprintln!("\nCannot audit the following crates because they are not from crates.io:");
            for crate_name in &foreign_crate_names {
                eprintln!(" - {}", crate_name);
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
