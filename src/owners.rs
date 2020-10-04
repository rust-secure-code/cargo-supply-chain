use crate::crates_io::*;
use serde::Deserialize;
use std::io::Result;

#[derive(Deserialize)]
struct OwnersResponse {
    users: Vec<OwnerData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OwnerData {
    id: u64,
    login: String,
    kind: OwnerKind,
    url: String,
    name: String,
    avatar: String,
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
enum OwnerKind {
    user,
    team,
}

pub fn owner_users(client: &mut ApiClient, crate_name: &str) -> Result<Vec<OwnerData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_user", crate_name);
    let data: OwnersResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.users)
}

pub fn owner_teams(client: &mut ApiClient, crate_name: &str) -> Result<Vec<OwnerData>> {
    let url = format!("https://crates.io/api/v1/crates/{}/owner_team", crate_name);
    let data: OwnersResponse = client.get(&url).call().into_json_deserialize()?;
    Ok(data.users)
}

pub fn owners(client: &mut ApiClient, crate_name: &str) -> Result<Vec<OwnerData>> {
    let mut users = owner_users(client, crate_name)?;
    let mut teams = owner_teams(client, crate_name)?;
    users.extend(teams.drain(..));
    Ok(users)
}