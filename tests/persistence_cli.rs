use std::{
    fs,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    config: PathBuf,
    data: PathBuf,
    locator: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rateq-cli-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let home = root.join("home");
        let config = root.join("config");
        let data = root.join("data");
        fs::create_dir_all(&home).unwrap();
        #[cfg(target_os = "macos")]
        let locator = home.join("Library/Application Support/RaTeQ/approved-location");
        #[cfg(not(target_os = "macos"))]
        let locator = config.join("rateq/approved-location");
        Self {
            root,
            home,
            config,
            data,
            locator,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rateq"))
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_DATA_HOME", &self.data)
            .output()
            .unwrap()
    }
    fn approve_fixture(&self) -> PathBuf {
        let custom = self.root.join("custom data");
        fs::create_dir_all(&custom).unwrap();
        fs::create_dir_all(self.locator.parent().unwrap()).unwrap();
        fs::write(&self.locator, custom.as_os_str().as_bytes()).unwrap();
        custom
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn standalone_commands_do_not_create_application_data() {
    let fixture = Fixture::new();
    for option in ["--help", "--version"] {
        assert!(fixture.run(&[option]).status.success());
    }
    assert!(!fixture.locator.exists());
    assert!(!fixture.config.exists());
    assert!(!fixture.data.exists());
    assert!(!fixture.home.join("Library").exists());
}

#[test]
fn approved_location_without_history_reports_e105() {
    let fixture = Fixture::new();
    fixture.approve_fixture();
    let output = fixture.run(&["--resume"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E105"));
}

#[test]
fn missing_resume_file_reports_e206_and_preserves_history() {
    let fixture = Fixture::new();
    let data = fixture.approve_fixture();
    let history = data.join("last-source");
    let source = fixture.root.join("missing.txt");
    fs::write(&history, source.as_os_str().as_bytes()).unwrap();
    let output = fixture.run(&["--resume"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E206"));
    assert_eq!(fs::read(history).unwrap(), source.as_os_str().as_bytes());
}

#[test]
fn invalid_resume_history_is_preserved_and_reported() {
    let fixture = Fixture::new();
    let data = fixture.approve_fixture();
    let history = data.join("last-source");
    fs::write(&history, b"relative/file.txt").unwrap();
    let output = fixture.run(&["--resume"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E208"));
    assert_eq!(fs::read(history).unwrap(), b"relative/file.txt");
}

#[test]
fn invalid_configuration_is_preserved_and_reported() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.locator.parent().unwrap()).unwrap();
    fs::write(&fixture.locator, b"relative/path").unwrap();
    let output = fixture.run(&["--resume"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E207"));
    assert_eq!(fs::read(&fixture.locator).unwrap(), b"relative/path");
    let conflict = fixture.run(&["--resume", "file.txt"]);
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("E102"));
}
