use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CratesCache, DownloadState};

pub fn update(max_age: std::time::Duration) {
    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();

    match cache.download(&mut client, max_age).unwrap() {
        DownloadState::Fresh => println!("No updates found"),
        DownloadState::Expired => println!("Successfully updated to the newest daily data dump."),
        DownloadState::Stale => {
            println!("Downloaded latest daily data dump.");
            println!("  Warning: it matches the previous version that was considered outdated.");
            std::process::exit(1);
        }
    }
}
