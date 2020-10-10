use crate::api_client::RateLimitedClient;
use crate::publishers::{PublisherData, PublisherKind};
use std::{collections::HashMap, fs, io, path::PathBuf};
use serde::{Serialize, Deserialize};
use libflate::gzip;

pub struct CratesCache {
    cache_dir: Option<CacheDir>,
    crates: Option<HashMap<String, Crate>>,
    crate_owners: Option<HashMap<u64, Vec<CrateOwner>>>,
    users: Option<HashMap<u64, User>>,
    teams: Option<HashMap<u64, Team>>,
    versions: Option<HashMap<(u64, String), Publisher>>,
}

struct CacheDir(PathBuf);

#[derive(Clone, Deserialize, Serialize)]
struct Crate {
    name: String,
    id: u64,
    repository: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CrateOwner {
    crate_id: u64,
    owner_id: u64,
    owner_kind: i32,
}

#[derive(Clone, Deserialize, Serialize)]
struct Publisher {
    crate_id: u64,
    published_by: u64,
}

#[derive(Clone, Deserialize, Serialize)]
struct Team {
    id: u64,
    avatar: Option<String>,
    login: String,
    name: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct User {
    id: u64,
    gh_avatar: Option<String>,
    gh_id: Option<String>,
    gh_login: String,
    name: Option<String>,
}

impl CratesCache {
    const CRATES_FS: &'static str = "crates.json";
    const CRATE_OWNERS_FS: &'static str = "crate_owners.json";
    const USERS_FS: &'static str = "users.json";
    const TEAMS_FS: &'static str = "teams.json";
    const VERSIONS_FS: &'static str = "versions.json";

    /// Open a crates cache.
    pub fn new() -> Self {
        CratesCache {
            cache_dir: Self::cache_dir().map(CacheDir),
            crates: None,
            crate_owners: None,
            users: None,
            teams: None,
            versions: None,
        }
    }

    fn cache_dir() -> Option<PathBuf> {
        let projects = directories::ProjectDirs::from("", "rust-secure-code", "cargo-supply-chain")?;
        Some(projects.cache_dir().to_owned())
    }

    /// Re-download the list from the data dumps.
    pub fn download(&mut self, client: &mut RateLimitedClient) -> io::Result<()> {
        let cache = self.cache_dir
            .as_ref()
            .ok_or(io::ErrorKind::NotFound)?;

        cache.validate_file_creation()?;

        let url = "https://static.crates.io/db-dump.tar.gz";
        let reader = client.get(url).call().into_reader();
        let ungzip = gzip::Decoder::new(reader)?;
        let mut archive = tar::Archive::new(ungzip);
        
        for file in archive.entries()? {
            if let Ok(entry) = file {
                if entry.path_bytes().ends_with(b"crate_owners.csv") {
                    let owners: Vec<CrateOwner> = read_csv_data(entry)?;
                    cache.store_multi_map(
                        &mut self.crate_owners,
                        Self::CRATE_OWNERS_FS,
                        owners.as_slice(),
                        &|owner| owner.crate_id,
                    )?;
                } else if entry.path_bytes().ends_with(b"crates.csv") {
                    let crates: Vec<Crate> = read_csv_data(entry)?;
                    cache.store_map(
                        &mut self.crates,
                        Self::CRATES_FS,
                        crates.as_slice(),
                        &|crate_| crate_.name.clone(),
                    )?;
                } else if entry.path_bytes().ends_with(b"users.csv") {
                    let users: Vec<User> = read_csv_data(entry)?;
                    cache.store_map(
                        &mut self.users,
                        Self::USERS_FS,
                        users.as_slice(),
                        &|user| user.id,
                    )?;
                } else if entry.path_bytes().ends_with(b"teams.csv") {
                    let teams: Vec<Team> = read_csv_data(entry)?;
                    cache.store_map(
                        &mut self.teams,
                        Self::TEAMS_FS,
                        teams.as_slice(),
                        &|team| team.id,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub fn publisher_users(&mut self, crate_name: &str) -> Option<Vec<PublisherData>> {
        let id = self.load_crates()?.get(crate_name)?.id;
        let owners = self.load_crate_owners()?.get(&id)?.clone();
        let users = self.load_users()?;
        let publisher = owners
            .into_iter()
            .filter(|owner| owner.owner_kind == 0)
            .filter_map(|owner: CrateOwner| {
                let user = users.get(&owner.owner_id)?;
                Some(PublisherData {
                    id: user.id,
                    avatar: user.gh_avatar.clone(),
                    url: None,
                    login: user.gh_login.clone(),
                    name: user.name.clone(),
                    kind: PublisherKind::user,
                })
            })
            .collect();
        Some(publisher)
    }

    pub fn publisher_teams(&mut self, crate_name: &str) -> Option<Vec<PublisherData>> {
        let id = self.load_crates()?.get(crate_name)?.id;
        let owners = self.load_crate_owners()?.get(&id)?.clone();
        let teams = self.load_teams()?;
        let publisher = owners
            .into_iter()
            .filter(|owner| owner.owner_kind == 1)
            .filter_map(|owner: CrateOwner| {
                let team = teams.get(&owner.owner_id)?;
                Some(PublisherData {
                    id: team.id,
                    avatar: team.avatar.clone(),
                    url: None,
                    login: team.login.clone(),
                    name: team.name.clone(),
                    kind: PublisherKind::team,
                })
            })
            .collect();
        Some(publisher)
    }

    fn load_crates(&mut self) -> Option<&HashMap<String, Crate>> {
        self.cache_dir.as_ref()?.load_cached(&mut self.crates, Self::CRATES_FS).ok()
    }

    fn load_crate_owners(&mut self) -> Option<&HashMap<u64, Vec<CrateOwner>>> {
        self.cache_dir.as_ref()?.load_cached(&mut self.crate_owners, Self::CRATE_OWNERS_FS).ok()
    }

    fn load_users(&mut self) -> Option<&HashMap<u64, User>> {
        self.cache_dir.as_ref()?.load_cached(&mut self.users, Self::USERS_FS).ok()
    }

    fn load_teams(&mut self) -> Option<&HashMap<u64, Team>> {
        self.cache_dir.as_ref()?.load_cached(&mut self.teams, Self::TEAMS_FS).ok()
    }

    fn load_versions(&mut self) -> Option<&HashMap<(u64, String), Publisher>> {
        self.cache_dir.as_ref()?.load_cached(&mut self.versions, Self::VERSIONS_FS).ok()
    }
}

fn read_csv_data<T: serde::de::DeserializeOwned>(from: impl io::Read)
    -> Result<Vec<T>, csv::Error>
{
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b',')
        .double_quote(true)
        .quoting(true)
        .from_reader(from);
    reader
        .deserialize()
        .collect()
}

impl CacheDir {
    fn validate_file_creation(&self) -> io::Result<()> {
        if !self.0.exists() {
            fs::create_dir_all(&self.0)?;
        }

        if !self.0.is_dir() {
            // Well. We certainly don't want to delete anything.
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        Ok(())
    }

    fn load_cached<'cache, T>(&self, cache: &'cache mut Option<T>, file: &str)
        -> io::Result<&'cache T>
    where
        T: serde::de::DeserializeOwned,
    {
        match cache {
            Some(datum) => Ok(datum),
            None => {
                let file = fs::File::open(self.0.join(file))?;
                let crates: T = serde_json::from_reader(file).unwrap();
                Ok(cache.get_or_insert(crates))
            }
        }
    }

    fn store_map<T, K>(&self,
        cache: &mut Option<HashMap<K, T>>,
        file: &str,
        entries: &[T],
        key_fn: &dyn Fn(&T) -> K,
    )
        -> io::Result<()>
    where
        T: Serialize + Clone,
        K: Serialize + Eq + std::hash::Hash,
    {
        let hashed: HashMap<K, _> = entries
            .iter()
            .map(|entry| (key_fn(entry), entry.clone()))
            .collect();
        *cache = None;
        let value = cache.get_or_insert(hashed);

        let out = fs::File::create(self.0.join(file))?;
        serde_json::to_writer(out, value)?;
        Ok(())
    }

    fn store_multi_map<T, K>(&self,
        cache: &mut Option<HashMap<K, Vec<T>>>,
        file: &str,
        entries: &[T],
        key_fn: &dyn Fn(&T) -> K,
    )
        -> io::Result<()>
    where
        T: Serialize + Clone,
        K: Serialize + Eq + std::hash::Hash,
    {
        let mut hashed: HashMap<K, _> = HashMap::new();
        entries
            .iter()
            .for_each(|entry| {
                let key = key_fn(entry);
                hashed
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(entry.clone())
            });
        *cache = None;
        let value = cache.get_or_insert(hashed);

        let out = fs::File::create(self.0.join(file))?;
        serde_json::to_writer(out, value)?;
        Ok(())
    }
}
