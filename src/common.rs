use anyhow::bail;
use cargo_metadata::{
    semver::VersionReq, CargoOpt::AllFeatures, CargoOpt::NoDefaultFeatures, Dependency,
    DependencyKind, Metadata, MetadataCommand, Package, PackageId,
};
use std::collections::{HashMap, HashSet};

pub use crate::cli::MetadataArgs;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(serde::Deserialize, serde::Serialize))]
pub enum PkgSource {
    Local,
    CratesIo,
    Foreign,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq, serde::Deserialize, serde::Serialize))]
pub struct SourcedPackage {
    pub source: PkgSource,
    pub package: Package,
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
    let mut other_options = Vec::new();
    if let Some(target) = args.target {
        other_options.push(format!("--filter-platform={}", target));
    }
    // `cargo-metadata` crate assumes we have a Vec of features,
    // but we really didn't want to parse it ourselves, so we pass the argument directly
    if let Some(features) = args.features {
        other_options.push(format!("--features={}", features));
    }
    command.other_options(other_options);
    command
}

pub fn sourced_dependencies(
    metadata_args: MetadataArgs,
) -> Result<Vec<SourcedPackage>, anyhow::Error> {
    let no_dev = metadata_args.no_dev;
    let command = metadata_command(metadata_args);
    let meta = match command.exec() {
        Ok(v) => v,
        Err(cargo_metadata::Error::CargoMetadata { stderr: e }) => bail!(e),
        Err(err) => bail!("Failed to fetch crate metadata!\n  {}", err),
    };

    sourced_dependencies_from_metadata(meta, no_dev)
}

fn sourced_dependencies_from_metadata(
    meta: Metadata,
    no_dev: bool,
) -> Result<Vec<SourcedPackage>, anyhow::Error> {
    let mut how: HashMap<PackageId, PkgSource> = HashMap::new();
    let mut what: HashMap<PackageId, Package> = meta
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

    if no_dev {
        (how, what) = extract_non_dev_dependencies(&mut how, &mut what);
    }

    let dependencies: Vec<_> = how
        .iter()
        .map(|(id, kind)| {
            let dep = what.get(id).cloned().unwrap();
            SourcedPackage {
                source: *kind,
                package: dep,
            }
        })
        .collect();

    Ok(dependencies)
}

#[derive(Eq, Hash, PartialEq)]
struct Dep {
    name: String,
    req: VersionReq,
}

impl Dep {
    fn from_cargo_metadata_dependency(dep: &Dependency) -> Self {
        Self {
            name: dep.name.clone(),
            req: dep.req.clone(),
        }
    }

    fn matches(&self, pkg: &Package) -> bool {
        self.name == pkg.name && self.req.matches(&pkg.version)
    }
}

/// Start with the `PkgSource::Local` packages, then iteratively add non-dev-dependencies until no more
/// packages can be added, and return the results.
///
/// Note that matching dependencies to packages is "best effort." The fields that Cargo uses to
/// determine a package's id are its name, version, and source:
/// https://github.com/rust-lang/cargo/blob/dd5134c7a59e3a3b8587f1ef04a930185d2ca503/src/cargo/core/package_id.rs#L29-L31
///
/// When matching dependencies to packages, we use the package's name and version, but not its source
/// (see [`Dep`]). Experiments suggest that source strings can vary. So comparing them seems risky.
/// Also, it is better to err on the side of inclusion.
fn extract_non_dev_dependencies(
    how: &mut HashMap<PackageId, PkgSource>,
    what: &mut HashMap<PackageId, Package>,
) -> (HashMap<PackageId, PkgSource>, HashMap<PackageId, Package>) {
    let mut how_new = HashMap::new();
    let mut what_new = HashMap::new();

    let mut ids = how
        .iter()
        .filter_map(|(id, source)| {
            if matches!(source, PkgSource::Local) {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    while !ids.is_empty() {
        let mut deps = HashSet::new();

        for id in ids.drain(..) {
            for dep in &what.get(&id).unwrap().dependencies {
                if dep.kind != DependencyKind::Development {
                    deps.insert(Dep::from_cargo_metadata_dependency(dep));
                }
            }

            how_new.insert(id.clone(), how.remove(&id).unwrap());
            what_new.insert(id.clone(), what.remove(&id).unwrap());
        }

        for pkg in what.values() {
            if deps.iter().any(|dep| dep.matches(pkg)) {
                ids.push(pkg.id.clone());
            }
        }
    }

    (how_new, what_new)
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

#[cfg(test)]
mod tests {
    use super::{sourced_dependencies_from_metadata, SourcedPackage};
    use cargo_metadata::{Metadata, MetadataCommand};
    use std::{
        cmp::Ordering,
        env::var,
        fs::{read_dir, read_to_string, write},
        path::Path,
    };

    #[test]
    fn deps() {
        for entry in read_dir("deps_tests").unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            let Some(prefix) = path
                .to_string_lossy()
                .strip_suffix(".metadata.json")
                .map(ToOwned::to_owned)
            else {
                continue;
            };

            let contents = read_to_string(&path).unwrap();

            // Help ensure private information is not leaked.
            assert!(var("HOME").map_or(true, |home| !contents.contains(&home)));

            let metadata = serde_json::from_str::<Metadata>(&contents).unwrap();

            for no_dev in [false, true] {
                let path = prefix.clone() + ".deps" + if no_dev { "_no_dev" } else { "" } + ".json";

                let mut deps_from_metadata =
                    sourced_dependencies_from_metadata(metadata.clone(), no_dev).unwrap();
                deps_from_metadata.sort_by(cmp_dep);

                if enabled("BLESS") {
                    let contents = serde_json::to_string_pretty(&deps_from_metadata).unwrap();
                    write(path, &contents).unwrap();
                    continue;
                }

                let mut deps_from_file = sourced_dependencies_from_file(&path);
                deps_from_file.sort_by(cmp_dep);

                assert_eq!(deps_from_file, deps_from_metadata);
            }
        }
    }

    // `cargo` has `snapbox` as a dev dependency. `snapbox` has `snapbox-macros` as a normal
    // dependency.

    #[test]
    fn cargo() {
        let deps = sourced_dependencies_from_file("deps_tests/cargo_0.70.1.deps.json");

        assert!(deps.iter().any(|dep| dep.package.name == "snapbox"));
        assert!(deps.iter().any(|dep| dep.package.name == "snapbox-macros"));
    }

    #[test]
    fn cargo_no_dev() {
        let deps = sourced_dependencies_from_file("deps_tests/cargo_0.70.1.deps_no_dev.json");

        assert!(deps.iter().all(|dep| dep.package.name != "snapbox"));
        assert!(deps.iter().all(|dep| dep.package.name != "snapbox-macros"));
    }

    #[test]
    fn snapbox() {
        let deps = sourced_dependencies_from_file("deps_tests/snapbox_0.4.11.deps.json");

        assert!(deps.iter().any(|dep| dep.package.name == "snapbox-macros"));
    }

    #[test]
    fn optional_dependency_excluded_when_not_activated() {
        let metadata = MetadataCommand::new()
            .current_dir("fixtures/optional_non_dev_dep")
            .exec()
            .unwrap();

        let deps = sourced_dependencies_from_metadata(metadata.clone(), false).unwrap();
        assert!(deps.iter().any(|dep| dep.package.name == "libz-rs-sys"));

        let deps_no_dev = sourced_dependencies_from_metadata(metadata, true).unwrap();
        assert!(!deps_no_dev
            .iter()
            .any(|dep| dep.package.name == "libz-rs-sys"));
    }

    fn sourced_dependencies_from_file(path: impl AsRef<Path>) -> Vec<SourcedPackage> {
        let contents = read_to_string(path).unwrap();
        serde_json::from_str::<Vec<SourcedPackage>>(&contents).unwrap()
    }

    fn cmp_dep(left: &SourcedPackage, right: &SourcedPackage) -> Ordering {
        left.package.id.cmp(&right.package.id)
    }

    fn enabled(key: &str) -> bool {
        var(key).is_ok_and(|value| value != "0")
    }
}
