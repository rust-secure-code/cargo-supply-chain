use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CacheState, CratesCache};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
    time::Duration,
};

use schemars::JsonSchema;

use crate::common::*;

#[derive(Deserialize)]
struct UsersResponse {
    users: Vec<PublisherData>,
}

#[derive(Deserialize)]
struct TeamsResponse {
    teams: Vec<PublisherData>,
}

/// Data about a single publisher received from a crates.io API endpoint
#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone)]
pub struct PublisherData {
    pub id: u64,
    pub login: String,
    pub kind: PublisherKind,
    pub url: Option<String>,
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

#[derive(JsonSchema, Serialize, Deserialize, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
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
        .get(&url)
        .call()
        .map_err(|e| io::Error::new(ErrorKind::Other, e))?;

    let mut count = 1;
    let mut wait = 5;
    while resp.status() != 200 && count <= attempts {
        eprintln!(
            "Failed retrieving {:?}, trying again in {} seconds, attempt {}/{}",
            url, wait, count, attempts
        );
        std::thread::sleep(std::time::Duration::from_secs(wait));

        resp = client
            .get(&url)
            .call()
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;

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
    let crates_io_names = crate_names_from_source(&dependencies, PkgSource::CratesIo);
    let mut client = RateLimitedClient::new();
    let mut cached = CratesCache::new();
    let using_cache = match cached.expire(max_age) {
        CacheState::Fresh => true,
        CacheState::Expired => {
            eprintln!(
                "\nIgnoring expired cache, older than {}.",
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
        match cached.age() {
            Some(age) => eprintln!(
                "\nUsing cached data. Cache age: {}",
                humantime::format_duration(age)
            ),
            None => unreachable!(),
        }
    } else {
        eprintln!("\nFetching publisher info from crates.io");
        eprintln!("This will take roughly 2 seconds per crate due to API rate limits");
    }
    for (ind, crate_name) in crates_io_names.iter().enumerate() {
        let cached_users = cached.publisher_users(crate_name);
        let cached_teams = cached.publisher_teams(crate_name);
        if let (Some(pub_users), Some(pub_teams)) = (cached_users, cached_teams) {
            // Progress output for downloading crates was meant as a progress bar.
            // We don't need it for cache, since it's fast anyway.
            // eprintln!(
            //     "Using cached data for \"{}\" ({}/{})",
            //     crate_name,
            //     i,
            //     crates_io_names.len()
            // );
            users.insert(crate_name.clone(), pub_users);
            teams.insert(crate_name.clone(), pub_teams);
        } else {
            eprintln!(
                "Fetching data for \"{}\" ({}/{})",
                crate_name,
                ind + 1,
                crates_io_names.len()
            );
            let pusers = publisher_users(&mut client, crate_name)?;
            users.insert(crate_name.clone(), pusers);
            let pteams = publisher_teams(&mut client, crate_name)?;
            teams.insert(crate_name.clone(), pteams);
        }
    }
    Ok((users, teams))
}
