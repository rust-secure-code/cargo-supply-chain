use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CratesCache, DownloadState};
use anyhow::bail;

pub fn update(max_age: std::time::Duration) -> anyhow::Result<()> {
    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();

    match cache.download(&mut client, max_age) {
        Ok(state) => match state {
            DownloadState::Fresh => eprintln!("No updates found"),
            DownloadState::Expired => {
                eprintln!("Successfully updated to the newest daily data dump.")
            }
            DownloadState::Stale => bail!("Downloaded latest daily data dump.\n  Warning: it matches the previous version that was considered outdated.")
        },
        Err(error) => bail!("Could not update to the latest daily data dump!\n{}", error)
    }
    Ok(())
}
