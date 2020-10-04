use crate::crates_io::*;
use serde::Deserialize;
use std::io::Result;

#[derive(Deserialize)]
struct UsersResponse {
    users: Vec<OwnerData>,
}

#[derive(Deserialize)]
struct TeamsResponse {
    teams: Vec<OwnerData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OwnerData {
    pub id: u64,
    pub login: String,
    pub kind: OwnerKind,
    pub url: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

impl PartialEq for OwnerData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for OwnerData {
    // holds for OwnerData because we're comparing u64 IDs, and it holds for u64
    fn assert_receiver_is_total_eq(&self) {}
}

impl PartialOrd for OwnerData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl Ord for OwnerData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum OwnerKind {
    user,
    team,
}

pub fn owner_users(client: &mut ApiClient, crate_name: &str) -> Result<Vec<OwnerData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_user", crate_name);
    let data: UsersResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.users)
}

pub fn owner_teams(client: &mut ApiClient, crate_name: &str) -> Result<Vec<OwnerData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_team", crate_name);
    let data: TeamsResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.teams)
}

pub fn owners(client: &mut ApiClient, crate_name: &str) -> Result<Vec<OwnerData>> {
    let mut users = owner_users(client, crate_name)?;
    let mut teams = owner_teams(client, crate_name)?;
    users.extend(teams.drain(..));
    Ok(users)
}
