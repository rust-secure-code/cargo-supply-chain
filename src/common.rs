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

pub fn sourced_dependencies(mut args: std::env::ArgsOs) -> Vec<SourcedPackage> {
    let mut extra_options: Vec<String> = Vec::new();
    while let Some(arg) = args.next() {
        match arg.into_string() {
            Ok(arg) => extra_options.push(arg),
            Err(arg) => bail_bad_arg(arg),
        }
    }

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

pub fn bail_bad_arg(arg: std::ffi::OsString) -> ! {
    eprintln!("Bad argument: {}", std::path::Path::new(&arg).display());
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