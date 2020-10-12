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
    let data: UsersResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.users)
}

pub fn publisher_teams(
    client: &mut RateLimitedClient,
    crate_name: &str,
) -> Result<Vec<PublisherData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_team", crate_name);
    let data: TeamsResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.teams)
}

pub fn fetch_owners_of_crates(
    dependencies: &[SourcedPackage],
) -> (
    HashMap<String, Vec<PublisherData>>,
    HashMap<String, Vec<PublisherData>>,
) {
    let crates_io_names = crate_names_from_source(&dependencies, PkgSource::CratesIo);
    let mut client = RateLimitedClient::new();
    let mut cached = CratesCache::new();
    let max_age = Duration::from_secs(48 * 3600);
    match cached.expire(max_age) {
        CacheState::Fresh => {}
        CacheState::Expired => {
            eprintln!("Ignoring expired cache, older than {}.", humantime::format_duration(max_age));
            eprintln!("  Run `cargo supply-chain update` to update it.");
        }
        CacheState::Unknown => {
            eprintln!("The `crates.io` cache was not found or it is invalid.");
            eprintln!("  Run `cargo supply-chain update` to generate it.");
        }
    }
    let mut users: HashMap<String, Vec<PublisherData>> = HashMap::new();
    let mut teams: HashMap<String, Vec<PublisherData>> = HashMap::new();
    eprintln!("\nFetching publisher info from crates.io");
    eprintln!("This will take roughly 2 seconds per crate due to API rate limits");
    for (i, crate_name) in crates_io_names.iter().enumerate() {
        let cached_users = cached.publisher_users(crate_name);
        let cached_teams = cached.publisher_teams(crate_name);
        if let (Some(pub_users), Some(pub_teams)) = (cached_users, cached_teams) {
            eprintln!(
                "Using cached data for \"{}\" ({}/{})",
                crate_name,
                i,
                crates_io_names.len()
            );
            users.insert(crate_name.clone(), pub_users);
            teams.insert(crate_name.clone(), pub_teams);
        } else {
            eprintln!(
                "Fetching data for \"{}\" ({}/{})",
                crate_name,
                i,
                crates_io_names.len()
            );
            users.insert(
                crate_name.clone(),
                publisher_users(&mut client, crate_name).unwrap(), //TODO: don't panic
            );
            teams.insert(
                crate_name.clone(),
                publisher_teams(&mut client, crate_name).unwrap(), //TODO: don't panic
            );
        }
    }
    (users, teams)
}
