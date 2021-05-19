use crate::crates_cache::{CratesCache, DownloadState};
use crate::{api_client::RateLimitedClient, err_exit};

pub fn update(max_age: std::time::Duration) {
    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();

    match cache.download(&mut client, max_age) {
        Ok(state) => match state {
            DownloadState::Fresh => eprintln!("No updates found"),
            DownloadState::Expired => {
                eprintln!("Successfully updated to the newest daily data dump.")
            }
            DownloadState::Stale => err_exit("Downloaded latest daily data dump.\n  Warning: it matches the previous version that was considered outdated.")
        },
        Err(error) => err_exit(format!("Could not update to the latest daily data dump!\n{}", error).as_str())
    }
}
