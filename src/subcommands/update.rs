use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CratesCache, DownloadState};
use anyhow::{bail, Context};

pub fn update(max_age: std::time::Duration) -> Result<(), anyhow::Error> {
    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();

    match cache.download(&mut client, max_age).context("Could not update to the latest daily data dump") {
        Ok(state) => match state {
            DownloadState::Fresh => eprintln!("No updates found"),
            DownloadState::Expired => {
                eprintln!("Successfully updated to the newest daily data dump.")
            }
            DownloadState::Stale => bail!("Downloaded latest daily data dump.\n  Warning: it matches the previous version that was considered outdated.")
        },
        Err(error) => bail!(error)
    }
    Ok(())
}
