use std::{
    env, fs,
    io::{self, Write},
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{DirBuilderExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

const LOCATOR_FILE: &str = "approved-location";
const HISTORY_FILE: &str = "last-source";

#[derive(Clone, Debug)]
pub struct Locations {
    pub default_data: PathBuf,
    pub locator: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Store {
    data_dir: PathBuf,
}

impl Locations {
    pub fn history_path(data_dir: &Path) -> PathBuf {
        data_dir.join(HISTORY_FILE)
    }
    pub fn from_environment() -> io::Result<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|value| value.is_absolute());
        #[cfg(target_os = "macos")]
        let (config_dir, default_data) = {
            let home = home.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "HOME is not an absolute path")
            })?;
            let data = home.join("Library/Application Support/RaTeQ");
            (data.clone(), data)
        };
        #[cfg(not(target_os = "macos"))]
        let (config_dir, default_data) = {
            (
                xdg_base(env::var_os("XDG_CONFIG_HOME"), home.as_deref(), ".config")?.join("rateq"),
                xdg_base(
                    env::var_os("XDG_DATA_HOME"),
                    home.as_deref(),
                    ".local/share",
                )?
                .join("rateq"),
            )
        };
        Ok(Self {
            default_data,
            locator: config_dir.join(LOCATOR_FILE),
        })
    }

    // This is deliberately read-only: a normal launch must not create application data.
    pub fn discover(&self) -> io::Result<Option<Store>> {
        match fs::read(&self.locator) {
            Ok(bytes) => {
                let path = decode_path(bytes, "approved location")?;
                Ok(Some(Store { data_dir: path }))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    // Call only after the user has approved both displayed locations.
    pub fn approve(&self, data_dir: &Path) -> io::Result<Store> {
        if !data_dir.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "data location must be an absolute path",
            ));
        }
        let parent = self
            .locator
            .parent()
            .ok_or_else(|| io::Error::other("location record has no parent"))?;
        let mut created = Vec::new();
        let result = (|| {
            create_private_dirs(data_dir, &mut created)?;
            create_private_dirs(parent, &mut created)?;
            atomic_write(&self.locator, data_dir.as_os_str().as_bytes())?;
            Ok(Store {
                data_dir: data_dir.to_path_buf(),
            })
        })();
        if result.is_err() {
            for directory in created.iter().rev() {
                // Remove only empty directories created by this attempt.
                let _ = fs::remove_dir(directory);
            }
        }
        result
    }
}

#[cfg(not(target_os = "macos"))]
fn xdg_base(
    value: Option<std::ffi::OsString>,
    home: Option<&Path>,
    fallback: &str,
) -> io::Result<PathBuf> {
    value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|path| path.join(fallback)))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "neither an absolute XDG directory nor HOME is available",
            )
        })
}

impl Store {
    pub fn last_source(&self) -> io::Result<Option<PathBuf>> {
        if !fs::metadata(&self.data_dir)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "application-data location is not a directory",
            ));
        }
        match fs::read(Locations::history_path(&self.data_dir)) {
            Ok(bytes) => decode_path(bytes, "saved source path").map(Some),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub fn record_source(&self, path: &Path) -> io::Result<()> {
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "saved source path must be absolute",
            ));
        }
        atomic_write(
            &Locations::history_path(&self.data_dir),
            path.as_os_str().as_bytes(),
        )
    }
}

fn decode_path(bytes: Vec<u8>, name: &str) -> io::Result<PathBuf> {
    if bytes.is_empty() || bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {name}"),
        ));
    }
    let path = PathBuf::from(std::ffi::OsString::from_vec(bytes));
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{name} is not absolute"),
        ));
    }
    Ok(path)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent"))?;
    for _ in 0..16 {
        let temporary = parent.join(format!(".rateq-{:016x}.tmp", fastrand::u64(..)));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = file
                    .write_all(bytes)
                    .and_then(|_| file.sync_all())
                    .and_then(|_| fs::rename(&temporary, path));
                if result.is_err() {
                    let _ = fs::remove_file(&temporary);
                }
                return result;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a temporary data file",
    ))
}

fn create_private_dirs(path: &Path, created: &mut Vec<PathBuf>) -> io::Result<()> {
    let missing: Vec<_> = path
        .ancestors()
        .take_while(|dir| !dir.exists())
        .map(Path::to_path_buf)
        .collect();
    for directory in missing.into_iter().rev() {
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        match builder.create(&directory) {
            Ok(()) => created.push(directory),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && directory.is_dir() => {}
            Err(e) => return Err(e),
        }
    }
    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "application-data location is not a directory",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn xdg_paths_work_without_home_and_relative_values_fall_back() {
        assert_eq!(
            xdg_base(Some("/opt/config".into()), None, ".config").unwrap(),
            PathBuf::from("/opt/config")
        );
        assert_eq!(
            xdg_base(
                Some("relative".into()),
                Some(Path::new("/home/user")),
                ".config"
            )
            .unwrap(),
            PathBuf::from("/home/user/.config")
        );
        assert!(xdg_base(None, None, ".config").is_err());
    }
    fn test_locations() -> (PathBuf, Locations) {
        let root = env::temp_dir().join(format!(
            "rateq-persistence-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let locations = Locations {
            default_data: root.join("data"),
            locator: root.join("config").join(LOCATOR_FILE),
        };
        (root, locations)
    }
    #[test]
    fn discovery_is_read_only_and_approval_remembers_custom_location() {
        let (root, locations) = test_locations();
        assert!(locations.discover().unwrap().is_none());
        assert!(!root.exists());
        let custom = root.join("custom location");
        let store = locations.approve(&custom).unwrap();
        assert!(locations.locator.is_file());
        assert!(store.last_source().unwrap().is_none());
        let source = root.join("my equation.txt");
        store.record_source(&source).unwrap();
        assert_eq!(
            locations
                .discover()
                .unwrap()
                .unwrap()
                .last_source()
                .unwrap(),
            Some(source)
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn malformed_record_is_not_treated_as_missing() {
        let (root, locations) = test_locations();
        fs::create_dir_all(locations.locator.parent().unwrap()).unwrap();
        fs::write(&locations.locator, b"relative/path").unwrap();
        assert_eq!(
            locations.discover().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_history_write_preserves_existing_history() {
        let (root, locations) = test_locations();
        let store = locations.approve(&locations.default_data).unwrap();
        let first = root.join("first.txt");
        store.record_source(&first).unwrap();
        assert!(store.record_source(Path::new("relative")).is_err());
        assert_eq!(store.last_source().unwrap(), Some(first));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_approval_removes_only_new_empty_directories() {
        let (root, mut locations) = test_locations();
        fs::create_dir_all(&root).unwrap();
        let blocker = root.join("not-a-directory");
        fs::write(&blocker, b"unchanged").unwrap();
        locations.locator = blocker.join("approved-location");
        assert!(locations.approve(&locations.default_data).is_err());
        assert!(!locations.default_data.exists());
        assert_eq!(fs::read(&blocker).unwrap(), b"unchanged");
        fs::remove_dir_all(root).unwrap();
    }
}
