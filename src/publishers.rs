use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CacheState, CratesCache};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self},
    time::Duration,
};

#[cfg(test)]
use schemars::JsonSchema;

use crate::common::{crate_names_from_source, PkgSource, SourcedPackage};

#[derive(Deserialize)]
struct UsersResponse {
    users: Vec<PublisherData>,
}

#[derive(Deserialize)]
struct TeamsResponse {
    teams: Vec<PublisherData>,
}

/// Data about a single publisher received from a crates.io API endpoint
#[cfg_attr(test, derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublisherData {
    pub id: u64,
    pub login: String,
    pub kind: PublisherKind,
    // URL is disabled because it's present in API responses but not in DB dumps,
    // so the output would vary inconsistent depending on data source
    //pub url: Option<String>,
    /// Display name. It is NOT guaranteed to be unique!
    pub name: Option<String>,
    /// Avatar image URL
    pub avatar: Option<String>,
}

impl PartialEq for PublisherData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PublisherData {
    // holds for PublisherData because we're comparing u64 IDs, and it holds for u64
    fn assert_receiver_is_total_eq(&self) {}
}

impl PartialOrd for PublisherData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl Ord for PublisherData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[cfg_attr(test, derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum PublisherKind {
    team,
    user,
}

pub fn publisher_users(
    client: &mut RateLimitedClient,
    crate_name: &str,
) -> Result<Vec<PublisherData>, io::Error> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_user", crate_name);
    let resp = get_with_retry(&url, client, 3)?;
    let data: UsersResponse = resp.into_json()?;
    Ok(data.users)
}

pub fn publisher_teams(
    client: &mut RateLimitedClient,
    crate_name: &str,
) -> Result<Vec<PublisherData>, io::Error> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_team", crate_name);
    let resp = get_with_retry(&url, client, 3)?;
    let data: TeamsResponse = resp.into_json()?;
    Ok(data.teams)
}

fn get_with_retry(
    url: &str,
    client: &mut RateLimitedClient,
    attempts: u8,
) -> Result<ureq::Response, io::Error> {
    let mut resp = client
        .get(url)
        .call()
        .map_err(io::Error::other)?;

    let mut count = 1;
    let mut wait = 5;
    while resp.status() != 200 && count <= attempts {
        eprintln!(
            "Failed retrieving {:?}, trying again in {} seconds, attempt {}/{}",
            url, wait, count, attempts
        );
        std::thread::sleep(std::time::Duration::from_secs(wait));

        resp = client
            .get(url)
            .call()
            .map_err(io::Error::other)?;

        count += 1;
        wait *= 3;
    }

    Ok(resp)
}

pub fn fetch_owners_of_crates(
    dependencies: &[SourcedPackage],
    max_age: Duration,
) -> Result<
    (
        BTreeMap<String, Vec<PublisherData>>,
        BTreeMap<String, Vec<PublisherData>>,
    ),
    io::Error,
> {
    let crates_io_names = crate_names_from_source(dependencies, PkgSource::CratesIo);
    let mut client = RateLimitedClient::new();
    let mut cached = CratesCache::new();
    let using_cache = match cached.expire(max_age) {
        CacheState::Fresh => true,
        CacheState::Expired => {
            eprintln!(
                "\nIgnoring expired cache, older than {}.",
                // we use humantime rather than indicatif because we take humantime input
                // and here we simply repeat it back to the user
                humantime::format_duration(max_age)
            );
            eprintln!("  Run `cargo supply-chain update` to update it.");
            false
        }
        CacheState::Unknown => {
            eprintln!("\nThe `crates.io` cache was not found or it is invalid.");
            eprintln!("  Run `cargo supply-chain update` to generate it.");
            false
        }
    };
    let mut users: BTreeMap<String, Vec<PublisherData>> = BTreeMap::new();
    let mut teams: BTreeMap<String, Vec<PublisherData>> = BTreeMap::new();

    if using_cache {
        let age = cached.age().unwrap();
        eprintln!(
            "\nUsing cached data. Cache age: {}",
            indicatif::HumanDuration(age)
        );
    } else {
        eprintln!("\nFetching publisher info from crates.io");
        eprintln!("This will take roughly 2 seconds per crate due to API rate limits");
    }

    let bar = indicatif::ProgressBar::new(crates_io_names.len() as u64)
    .with_prefix("Preparing")
    .with_style(
        indicatif::ProgressStyle::default_bar()
        .template("{prefix:>12.bright.cyan} [{bar:27}] {pos:>4}/{len:4} ETA {eta:3} - {msg:.cyan}").unwrap()
        .progress_chars("=> ")
    );

    for (i, crate_name) in crates_io_names.iter().enumerate() {
        bar.set_message(crate_name.clone());
        bar.set_position((i + 1) as u64);
        let cached_users = cached.publisher_users(crate_name);
        let cached_teams = cached.publisher_teams(crate_name);
        if let (Some(pub_users), Some(pub_teams)) = (cached_users, cached_teams) {
            bar.set_prefix("Loading cache");
            users.insert(crate_name.clone(), pub_users);
            teams.insert(crate_name.clone(), pub_teams);
        } else {
            // Handle crates not found in the cache by fetching live data for them
            bar.set_prefix("Downloading");
            let pusers = publisher_users(&mut client, crate_name)?;
            users.insert(crate_name.clone(), pusers);
            let pteams = publisher_teams(&mut client, crate_name)?;
            teams.insert(crate_name.clone(), pteams);
        }
    }
    Ok((users, teams))
}
