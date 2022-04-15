use std::{path::PathBuf};

use tokei::{Config, Languages, LanguageType};

use crate::{common::*, MetadataArgs};

pub fn lines(metadata_args: MetadataArgs) -> Result<(), std::io::Error> {
    // we don't actually need sources but I didn't want to make another function just for this
    let dependencies = sourced_dependencies(metadata_args);
    let mut packages: Vec<String> = Vec::new();
    let mut code_dirs: Vec<PathBuf> = Vec::new();
    for package in dependencies.into_iter().map(|p| p.package) {
        packages.push(package.name);
        assert!((&package.manifest_path).ends_with("Cargo.toml"));
        let code_dir = package.manifest_path.parent().unwrap();
        code_dirs.push(code_dir.to_owned());
    }
    
    let config = Config::default();
    let mut languages = Languages::new();
    // FIXME: tokei will treat `code_dirs` as globs
    // https://github.com/XAMPPRocky/tokei/issues/906
    languages.get_statistics(&code_dirs, &[], &config);
    println!("{:?}", languages);
    Ok(())
}