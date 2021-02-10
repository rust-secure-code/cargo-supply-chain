use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CratesCache, DownloadState};

pub fn update(max_age: std::time::Duration) {
    eprintln!("Note: this will download large amounts of data, roughtly 250Mb.");
    eprintln!("On a slow network this will take a while.");

    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();

    match cache.download(&mut client, max_age) {
        Ok(state) => match state {
            DownloadState::Fresh => eprintln!("No updates found"),
            DownloadState::Expired => {
                eprintln!("Successfully updated to the newest daily data dump.")
            }
            DownloadState::Stale => {
                eprintln!("Downloaded latest daily data dump.");
                eprintln!(
                    "  Warning: it matches the previous version that was considered outdated."
                );
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("Could not update to the latest daily data dump!\n{}", error);
            std::process::exit(1);
        }
    }
}
