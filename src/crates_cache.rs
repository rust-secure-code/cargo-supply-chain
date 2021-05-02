use crate::api_client::RateLimitedClient;
use crate::publishers::{PublisherData, PublisherKind};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::iter::FromIterator;
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::{self, ErrorKind},
    mem,
    path::PathBuf,
    time::Duration,
    time::SystemTimeError,
};

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
            directories_next::ProjectDirs::from("", "rust-secure-code", "cargo-supply-chain")?;
        Some(projects.cache_dir().to_owned())
    }

    /// Re-download the list from the data dumps.
    pub fn download(
        &mut self,
        client: &mut RateLimitedClient,
        max_age: Duration,
    ) -> Result<DownloadState, io::Error> {
        let bar = indicatif::ProgressBar::new(!0)
            .with_prefix("Downloading")
            .with_style(
                indicatif::ProgressStyle::default_spinner()
                    .template("{prefix:>12.bright.cyan} {spinner} {msg:.cyan}"),
            )
            .with_message("preparing");

        let remembered_etag;
        let response = {
            let mut request = client.get(Self::DUMP_URL);
            if let Some(meta) = self.load_metadata() {
                remembered_etag = meta.etag.clone();
                // See if we can consider the resource not-yet-stale.
                if let Some(true) = meta.validate(max_age) {
                    if let Some(etag) = meta.etag.as_ref() {
                        request = request.set("if-none-match", &etag);
                    }
                }
            } else {
                remembered_etag = None;
            }
            request.call()
        }
        .map_err(|e| io::Error::new(ErrorKind::Other, e))?;

        // Not modified.
        if response.status() == 304 {
            bar.finish_and_clear();
            return Ok(DownloadState::Fresh);
        }

        if let Some(length) = response
            .header("content-length")
            .and_then(|l| l.parse().ok())
        {
            bar.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{prefix:>12.bright.cyan} [{bar:27}] {bytes:>9}/{total_bytes:9}  {bytes_per_sec}  ETA {eta:4} - {msg:.cyan}")
                    .progress_chars("=> "));
            bar.set_length(length);
        } else {
            bar.println("Length unspecified, expect at least 250MiB");
            bar.set_style(indicatif::ProgressStyle::default_spinner().template(
                "{prefix:>12.bright.cyan} {spinner} {bytes:>9} {bytes_per_sec} - {msg:.cyan}",
            ));
        }

        let etag = response.header("etag").map(String::from);
        let reader = bar.wrap_read(response.into_reader());
        let ungzip = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(ungzip);

        let cache_dir = CratesCache::cache_dir().ok_or(ErrorKind::NotFound)?;
        let mut cache_updater = CacheUpdater::new(cache_dir)?;
        let required_files = BTreeSet::from_iter(
            [
                Self::CRATE_OWNERS_FS,
                Self::CRATES_FS,
                Self::USERS_FS,
                Self::TEAMS_FS,
                Self::METADATA_FS,
            ]
            .iter()
            .map(|x| x.to_string()),
        );
        for file in archive.entries()? {
            if let Ok(entry) = file {
                if let Ok(path) = entry.path() {
                    if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                        bar.set_message(name.to_string());
                    }
                }
                if entry.path_bytes().ends_with(b"crate_owners.csv") {
                    let owners: Vec<CrateOwner> = read_csv_data(entry)?;
                    cache_updater.store_multi_map(
                        &mut self.crate_owners,
                        Self::CRATE_OWNERS_FS,
                        owners.as_slice(),
                        &|owner| owner.crate_id,
                    )?;
                } else if entry.path_bytes().ends_with(b"crates.csv") {
                    let crates: Vec<Crate> = read_csv_data(entry)?;
                    cache_updater.store_map(
                        &mut self.crates,
                        Self::CRATES_FS,
                        crates.as_slice(),
                        &|crate_| crate_.name.clone(),
                    )?;
                } else if entry.path_bytes().ends_with(b"users.csv") {
                    let users: Vec<User> = read_csv_data(entry)?;
                    cache_updater.store_map(
                        &mut self.users,
                        Self::USERS_FS,
                        users.as_slice(),
                        &|user| user.id,
                    )?;
                } else if entry.path_bytes().ends_with(b"teams.csv") {
                    let teams: Vec<Team> = read_csv_data(entry)?;
                    cache_updater.store_map(
                        &mut self.teams,
                        Self::TEAMS_FS,
                        teams.as_slice(),
                        &|team| team.id,
                    )?;
                } else if entry.path_bytes().ends_with(b"metadata.json") {
                    let meta: Metadata = serde_json::from_reader(entry)?;
                    cache_updater.store(
                        &mut self.metadata,
                        Self::METADATA_FS,
                        MetadataStored {
                            timestamp: meta.timestamp,
                            etag: etag.clone(),
                        },
                    )?;
                } else {
                    // This was not a file with a filename we actually use.
                    // Check if we've obtained all the files we need.
                    // If yes, we can end the download early.
                    // This saves hundreds of megabytes of traffic.
                    if required_files.is_subset(&cache_updater.staged_files) {
                        break;
                    }
                }
            }
        }
        // Now that we've successfully downloaded and stored everything,
        // replace the old cache contents with the new one.
        cache_updater.commit()?;

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

    pub fn age(&mut self) -> Option<Duration> {
        match self.load_metadata() {
            Some(meta) => meta.age().ok(),
            None => None,
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
    fn load_cached<'cache, T>(
        &self,
        cache: &'cache mut Option<T>,
        file: &str,
    ) -> Result<&'cache T, io::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match cache {
            Some(datum) => Ok(datum),
            None => {
                let file = fs::File::open(self.0.join(file))?;
                let reader = io::BufReader::new(file);
                let crates: T = serde_json::from_reader(reader).unwrap();
                Ok(cache.get_or_insert(crates))
            }
        }
    }
}

/// Implements a two-phase transactional update mechanism:
/// you can store data, but it will not overwrite previous data until you call `commit()`
struct CacheUpdater {
    dir: PathBuf,
    staged_files: BTreeSet<String>,
}

/// Creates the cache directory if it doesn't exist.
/// Returns an error if creation fails.
impl CacheUpdater {
    fn new(dir: PathBuf) -> Result<Self, io::Error> {
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        if !dir.is_dir() {
            // Well. We certainly don't want to delete anything.
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        Ok(Self {
            dir,
            staged_files: BTreeSet::new(),
        })
    }

    /// Commits to disk any changes that you have staged via the `store()` function.
    fn commit(&mut self) -> io::Result<()> {
        let mut uncommitted_files = mem::replace(&mut self.staged_files, BTreeSet::new());
        let metadata_file = uncommitted_files.take(CratesCache::METADATA_FS);
        for file in uncommitted_files {
            let source = self.dir.join(&file).with_extension("part");
            let destination = self.dir.join(&file);
            fs::rename(source, destination)?;
        }
        // metadata_file is special since it contains the timestamp for the cache.
        // We will only commit it and update the timestamp if updating everything else succeeds.
        // Otherwise it would be possible to create a partially updated cache that's considered fresh.
        if let Some(file) = metadata_file {
            let source = self.dir.join(&file).with_extension("part");
            let destination = self.dir.join(&file);
            fs::rename(source, destination)?;
        }
        Ok(())
    }

    /// Does not overwrite existing data until `commit()` is called.
    /// If you do not call `commit()` after this, the on-disk cache will not be actually updated!
    fn store<T>(&mut self, cache: &mut Option<T>, file: &str, value: T) -> Result<(), io::Error>
    where
        T: Serialize,
    {
        *cache = None;
        let value = cache.get_or_insert(value);

        self.staged_files.insert(file.to_owned());
        let out_path = self.dir.join(file).with_extension("part");
        let out_file = fs::File::create(out_path)?;
        let out = io::BufWriter::new(out_file);
        serde_json::to_writer(out, value)?;
        Ok(())
    }

    fn store_map<T, K>(
        &mut self,
        cache: &mut Option<HashMap<K, T>>,
        file: &str,
        entries: &[T],
        key_fn: &dyn Fn(&T) -> K,
    ) -> Result<(), io::Error>
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
        &mut self,
        cache: &mut Option<HashMap<K, Vec<T>>>,
        file: &str,
        entries: &[T],
        key_fn: &dyn Fn(&T) -> K,
    ) -> Result<(), io::Error>
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
