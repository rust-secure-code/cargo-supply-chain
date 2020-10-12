use crate::api_client::RateLimitedClient;
use crate::common::*;
use crate::crates_cache::{CratesCache, DownloadState};

pub fn update(mut args: std::env::ArgsOs) {
    if let Some(arg) = args.next() {
        bail_unknown_subcommand_arg("update", arg)
    }

    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();
    let stale = std::time::Duration::from_secs(48 * 3600);
    match cache.download(&mut client, stale).unwrap() {
        DownloadState::Fresh => println!("No updates found"),
        DownloadState::Expired => println!("Successfully updated to the newest daily data dump."),
        DownloadState::Stale => {
            println!("Downloaded latest daily data dump.");
            println!("  Warning: it matches the previous version that was considered outdated.");
        }
    }
}
