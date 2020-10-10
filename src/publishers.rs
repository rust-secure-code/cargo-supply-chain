use crate::crates_io::*;
use serde::Deserialize;
use std::io::Result;

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

pub fn publisher_users(client: &mut ApiClient, crate_name: &str) -> Result<Vec<PublisherData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_user", crate_name);
    let data: UsersResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.users)
}

pub fn publisher_teams(client: &mut ApiClient, crate_name: &str) -> Result<Vec<PublisherData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_team", crate_name);
    let data: TeamsResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.teams)
}
