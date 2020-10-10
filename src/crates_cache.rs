use crate::api_client::RateLimitedClient;
use std::{fs, io, path::PathBuf};
use libflate::gzip;

pub struct CratesCache {
    cache_dir: Option<PathBuf>,
}

impl CratesCache {
    /// Open a crates cache.
    pub fn new() -> Self {
        CratesCache {
            cache_dir: Self::cache_dir()
        }
    }

    fn cache_dir() -> Option<PathBuf> {
        let projects = directories::ProjectDirs::from("", "rust-secure-code", "cargo-supply-chain")?;
        Some(projects.cache_dir().to_owned())
    }

    /// Re-download the list from the data dumps.
    pub fn download(&mut self, client: &mut RateLimitedClient) -> io::Result<()> {
        if let Some(target_dir) = &self.cache_dir {
            if !target_dir.exists() {
                fs::create_dir_all(target_dir)?;
            }

            if !target_dir.is_dir() {
                // Well. We certainly don't want to delete anything.
                return Err(io::ErrorKind::AlreadyExists.into());
            }
        }

        let url = "https://static.crates.io/db-dump.tar.gz";
        let reader = client.get(url).call().into_reader();
        let ungzip = gzip::Decoder::new(reader)?;
        let archive = tar::Archive::new(ungzip);
        
        todo!()
    }

    pub fn publisher_by_name(&self, crate_name: &str) -> Option<String> {
        todo!()
    }

    pub fn publisher_by_version(&self, crate_name: &str, version: &str) -> Option<String> {
        todo!()
    }
}
