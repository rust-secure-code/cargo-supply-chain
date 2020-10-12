use crate::api_client::RateLimitedClient;
use crate::common::*;
use crate::crates_cache::{CratesCache, DownloadState};

pub fn update(mut args: std::env::ArgsOs) {
    let mut max_age = std::time::Duration::from_secs(48 * 3600);

    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("--cache-max-age") => {
                max_age = get_argument(arg, &mut args, |age| {
                    humantime::parse_duration(&age)
                });
            }
            _ => bail_unknown_subcommand_arg("update", arg)
        }
    }

    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();

    match cache.download(&mut client, max_age).unwrap() {
        DownloadState::Fresh => println!("No updates found"),
        DownloadState::Expired => println!("Successfully updated to the newest daily data dump."),
        DownloadState::Stale => {
            println!("Downloaded latest daily data dump.");
            println!("  Warning: it matches the previous version that was considered outdated.");
        }
    }
}
