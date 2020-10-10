use crate::api_client::RateLimitedClient;
use crate::common::*;
use crate::crates_cache::CratesCache;

pub fn update(mut args: std::env::ArgsOs) {
    if let Some(arg) = args.next() {
        bail_unknown_subcommand_arg("update", arg)
    }

    let mut cache = CratesCache::new();
    let mut client = RateLimitedClient::new();
    cache.download(&mut client).unwrap();
}
