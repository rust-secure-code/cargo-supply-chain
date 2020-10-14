use crate::api_client::RateLimitedClient;
use crate::publishers::{PublisherData, PublisherKind};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, path::PathBuf, time::Duration, time::SystemTimeError};

pub struct CratesCache {
    cache_dir: Option<CacheDir>,
    metadata: Option<MetadataStored>,
    crates: Option<HashMap<String, Crate>>,
    crate_owners: Option<HashMap<u64, Vec<CrateOwner>>>,
    users: Option<HashMap<u64, User>>,
    teams: Option<HashMap<u64, Team>>,
    versions: Option<HashMap<(u64, String), Publisher>>,
}

pub enum CacheState {
    Fresh,
    Expired,
    Unknown,
}

pub enum DownloadState {
    /// The tag still matched and resource was not stale.
    Fresh,
    /// There was a newer resource.
    Expired,
    /// We forced the download of an update.
    Stale,
}

pub enum AgeError {
    InvalidCache,
    CacheFromTheFuture(SystemTimeError),
}

impl From<SystemTimeError> for AgeError {
    fn from(err: SystemTimeError) -> Self {
        AgeError::CacheFromTheFuture(err)
    }
}

struct CacheDir(PathBuf);

#[derive(Clone, Deserialize, Serialize)]
struct Metadata {
    #[serde(with = "humantime_serde")]
    timestamp: std::time::SystemTime,
}

#[derive(Clone, Deserialize, Serialize)]
struct MetadataStored {
    #[serde(with = "humantime_serde")]
    timestamp: std::time::SystemTime,
    #[serde(default)]
    etag: Option<String>,
}

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
    const METADATA_FS: &'static str = "metadata.json";
    const CRATES_FS: &'static str = "crates.json";
    const CRATE_OWNERS_FS: &'static str = "crate_owners.json";
    const USERS_FS: &'static str = "users.json";
    const TEAMS_FS: &'static str = "teams.json";
    const VERSIONS_FS: &'static str = "versions.json";

    const DUMP_URL: &'static str = "https://static.crates.io/db-dump.tar.gz";

    /// Open a crates cache.
    pub fn new() -> Self {
        CratesCache {
            cache_dir: Self::cache_dir().map(CacheDir),
            metadata: None,
            crates: None,
            crate_owners: None,
            users: None,
            teams: None,
            versions: None,
        }
    }

    fn cache_dir() -> Option<PathBuf> {
        let projects =
            directories::ProjectDirs::from("", "rust-secure-code", "cargo-supply-chain")?;
        Some(projects.cache_dir().to_owned())
    }

    /// Re-download the list from the data dumps.
    pub fn download(
        &mut self,
        client: &mut RateLimitedClient,
        max_age: Duration,
    ) -> io::Result<DownloadState> {
        let cache = self.cache_dir.as_ref().ok_or(io::ErrorKind::NotFound)?;
        cache.validate_file_creation()?;

        let remembered_etag;
        let response = {
            let mut request = client.get(Self::DUMP_URL);
            if let Some(meta) = self.load_metadata() {
                remembered_etag = meta.etag.clone();
                // See if we can consider the resource not-yet-stale.
                if let Some(true) = meta.validate(max_age) {
                    if let Some(etag) = meta.etag.as_ref() {
                        request.set("if-none-match", &etag);
                    }
                }
            } else {
                remembered_etag = None;
            }
            request.call()
        };

        // Not modified.
        if response.status() == 304 {
            return Ok(DownloadState::Fresh);
        }

        let etag = response.header("etag").map(String::from);
        let reader = response.into_reader();
        let ungzip = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(ungzip);

        let cache = self.cache_dir.as_ref().ok_or(io::ErrorKind::NotFound)?;
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
                } else if entry.path_bytes().ends_with(b"metadata.json") {
                    let meta: Metadata = serde_json::from_reader(entry)?;
                    cache.store(
                        &mut self.metadata,
                        Self::METADATA_FS,
                        MetadataStored {
                            timestamp: meta.timestamp,
                            etag: etag.clone(),
                        },
                    )?;
                }
            }
        }

        // If we get here, we had no etag or the etag mismatched or we forced a download due to
        // stale data. Catch the last as it means the crates.io daily dumps were not updated.
        if remembered_etag == etag {
            Ok(DownloadState::Stale)
        } else {
            Ok(DownloadState::Expired)
        }
    }

    pub fn expire(&mut self, max_age: Duration) -> CacheState {
        match self.validate(max_age) {
            // Still fresh.
            Some(true) => CacheState::Fresh,
            // There was no valid meta data. Consider expired for safety.
            None => {
                self.cache_dir = None;
                CacheState::Unknown
            }
            Some(false) => {
                self.cache_dir = None;
                CacheState::Expired
            }
        }
    }

    pub fn age(&mut self) -> Result<Duration, AgeError> {
        match self.load_metadata() {
            Some(meta) => Ok(meta.age()?),
            None => Err(AgeError::InvalidCache),
        }
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

    fn validate(&mut self, max_age: Duration) -> Option<bool> {
        let meta = self.load_metadata()?;
        meta.validate(max_age)
    }

    fn load_metadata(&mut self) -> Option<&MetadataStored> {
        self.cache_dir
            .as_ref()?
            .load_cached(&mut self.metadata, Self::METADATA_FS)
            .ok()
    }

    fn load_crates(&mut self) -> Option<&HashMap<String, Crate>> {
        self.cache_dir
            .as_ref()?
            .load_cached(&mut self.crates, Self::CRATES_FS)
            .ok()
    }

    fn load_crate_owners(&mut self) -> Option<&HashMap<u64, Vec<CrateOwner>>> {
        self.cache_dir
            .as_ref()?
            .load_cached(&mut self.crate_owners, Self::CRATE_OWNERS_FS)
            .ok()
    }

    fn load_users(&mut self) -> Option<&HashMap<u64, User>> {
        self.cache_dir
            .as_ref()?
            .load_cached(&mut self.users, Self::USERS_FS)
            .ok()
    }

    fn load_teams(&mut self) -> Option<&HashMap<u64, Team>> {
        self.cache_dir
            .as_ref()?
            .load_cached(&mut self.teams, Self::TEAMS_FS)
            .ok()
    }

    fn load_versions(&mut self) -> Option<&HashMap<(u64, String), Publisher>> {
        self.cache_dir
            .as_ref()?
            .load_cached(&mut self.versions, Self::VERSIONS_FS)
            .ok()
    }
}

fn read_csv_data<T: serde::de::DeserializeOwned>(
    from: impl io::Read,
) -> Result<Vec<T>, csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b',')
        .double_quote(true)
        .quoting(true)
        .from_reader(from);
    reader.deserialize().collect()
}

impl MetadataStored {
    fn validate(&self, max_age: Duration) -> Option<bool> {
        match self.age() {
            Ok(duration) => Some(duration < max_age),
            Err(_) => None,
        }
    }

    pub fn age(&self) -> Result<Duration, SystemTimeError> {
        self.timestamp.elapsed()
    }
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

    fn load_cached<'cache, T>(
        &self,
        cache: &'cache mut Option<T>,
        file: &str,
    ) -> io::Result<&'cache T>
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

    fn store<T>(&self, cache: &mut Option<T>, file: &str, value: T) -> io::Result<()>
    where
        T: Serialize,
    {
        *cache = None;
        let value = cache.get_or_insert(value);

        let out = fs::File::create(self.0.join(file))?;
        serde_json::to_writer(out, value)?;
        Ok(())
    }

    fn store_map<T, K>(
        &self,
        cache: &mut Option<HashMap<K, T>>,
        file: &str,
        entries: &[T],
        key_fn: &dyn Fn(&T) -> K,
    ) -> io::Result<()>
    where
        T: Serialize + Clone,
        K: Serialize + Eq + std::hash::Hash,
    {
        let hashed: HashMap<K, _> = entries
            .iter()
            .map(|entry| (key_fn(entry), entry.clone()))
            .collect();
        self.store(cache, file, hashed)
    }

    fn store_multi_map<T, K>(
        &self,
        cache: &mut Option<HashMap<K, Vec<T>>>,
        file: &str,
        entries: &[T],
        key_fn: &dyn Fn(&T) -> K,
    ) -> io::Result<()>
    where
        T: Serialize + Clone,
        K: Serialize + Eq + std::hash::Hash,
    {
        let mut hashed: HashMap<K, _> = HashMap::new();
        entries.iter().for_each(|entry| {
            let key = key_fn(entry);
            hashed
                .entry(key)
                .or_insert_with(Vec::new)
                .push(entry.clone())
        });
        self.store(cache, file, hashed)
    }
}
