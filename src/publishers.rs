use std::io::{Error, ErrorKind};

use crate::api_client::RateLimitedClient;
use crate::crates_cache::{CacheState, CratesCache};
use serde::Deserialize;
use std::{collections::HashMap, io::Result, time::Duration};

use crate::common::*;

#[derive(Deserialize)]
struct UsersResponse {
    users: Vec<PublisherData>,
}

#[derive(Deserialize)]
struct TeamsResponse {
    teams: Vec<PublisherData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PublisherData {
    pub id: u64,
    pub login: String,
    pub kind: PublisherKind,
    pub url: Option<String>,
    pub name: Option<String>,
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

#[derive(Deserialize, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum PublisherKind {
    team,
    user,
}

pub fn publisher_users(
    client: &mut RateLimitedClient,
    crate_name: &str,
) -> Result<Vec<PublisherData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_user", crate_name);
    if let Some(resp) = retry_get(&url, client, 3) {
        let data: UsersResponse = resp.into_json_deserialize()?;
        Ok(data.users)
    } else {
        Err(Error::new(
            ErrorKind::ConnectionReset,
            "Failed to retrieve publisher users",
        ))
    }
}

pub fn publisher_teams(
    client: &mut RateLimitedClient,
    crate_name: &str,
) -> Result<Vec<PublisherData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_team", crate_name);
    if let Some(resp) = retry_get(&url, client, 3) {
        let data: TeamsResponse = resp.into_json_deserialize()?;
        Ok(data.teams)
    } else {
        Err(Error::new(
            ErrorKind::ConnectionReset,
            "Failed to retrieve publisher teams",
        ))
    }
}

fn retry_get(url: &String, client: &mut RateLimitedClient, attempts: u8) -> Option<ureq::Response> {
    let mut resp = client.get(&url).call();
    let mut count = 1;
    let mut wait = 10;
    while resp.status() != 200 && count <= attempts {
        eprintln!(
            "Failed retrieving {:?}, trying again in {} seconds, attempt {}/{}",
            url, wait, count, attempts
        );
        std::thread::sleep(std::time::Duration::from_secs(1));
        resp = client.get(&url).call();
        count += 1;
        wait *= 3;
    }
    if resp.status() == 200 {
        Some(resp)
    } else {
        None
    }
}

pub fn fetch_owners_of_crates(
    dependencies: &[SourcedPackage],
    max_age: Duration,
) -> (
    HashMap<String, Vec<PublisherData>>,
    HashMap<String, Vec<PublisherData>>,
) {
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
    let mut users: HashMap<String, Vec<PublisherData>> = HashMap::new();
    let mut teams: HashMap<String, Vec<PublisherData>> = HashMap::new();

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
    for (i, crate_name) in crates_io_names.iter().enumerate() {
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
                i,
                crates_io_names.len()
            );
            if let Ok(pusers) = publisher_users(&mut client, crate_name) {
                users.insert(crate_name.clone(), pusers);
            }
            if let Ok(pteams) = publisher_teams(&mut client, crate_name) {
                teams.insert(crate_name.clone(), pteams);
            }
        }
    }
    (users, teams)
}
