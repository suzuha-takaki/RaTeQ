use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Source,
    Png,
    Svg,
}
impl FileType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Png => "PNG",
            Self::Svg => "SVG",
        }
    }
    fn index(self) -> usize {
        match self {
            Self::Source => 0,
            Self::Png => 1,
            Self::Svg => 2,
        }
    }
    fn extension(self) -> Option<&'static str> {
        match self {
            Self::Source => None,
            Self::Png => Some("png"),
            Self::Svg => Some("svg"),
        }
    }
}

pub struct Files {
    cwd: PathBuf,
    launch: Option<PathBuf>,
    current: Option<PathBuf>,
    unnamed_stem: String,
    acknowledged: [Option<PathBuf>; 3],
}
impl Files {
    pub fn new(cwd: PathBuf, launch: Option<PathBuf>) -> Self {
        let launch = launch.map(|p| absolute(&cwd, &p));
        Self {
            cwd,
            current: launch.clone(),
            launch,
            unnamed_stem: generated_stem(include_str!("../assets/names.toml")),
            acknowledged: Default::default(),
        }
    }
    pub fn default_path(&self, kind: FileType) -> String {
        let mut path = self
            .current
            .clone()
            .unwrap_or_else(|| self.cwd.join(format!("{}.txt", self.unnamed_stem)));
        if let Some(ext) = kind.extension() {
            path.set_extension(ext);
        }
        path.to_string_lossy().into_owned()
    }
    pub fn validate(
        &self,
        text: &str,
        kind: FileType,
    ) -> Result<(PathBuf, Vec<&'static str>), &'static str> {
        if text.is_empty() || text.ends_with('/') {
            return Err("E203 ERROR: Enter a file path");
        }
        let path = absolute(&self.cwd, Path::new(text));
        if path.is_dir() || path.file_name().is_none() {
            return Err("E203 ERROR: Enter a file path");
        }
        if let Some(ext) = kind.extension() {
            if path.extension().and_then(|s| s.to_str()) != Some(ext) {
                return Err(
                    "E202 ERROR: The file extension does not match the selected export format",
                );
            }
        }
        let parent = path.parent().ok_or("E203 ERROR: Enter a file path")?;
        if !parent.is_dir() {
            return Err("E204 ERROR: The destination directory does not exist");
        }
        let dest = normalized(&path)
            .map_err(|_| "E204 ERROR: The destination directory does not exist")?;
        let mut warnings = Vec::new();
        if path.exists() {
            warnings.push("W201 WARNING: This file already exists and will be overwritten");
        }
        if text.contains(' ') {
            warnings.push("W202 WARNING: This path includes a space");
        }
        let acknowledged = self.acknowledged[kind.index()].as_ref() == Some(&dest);
        if !acknowledged {
            let differs = |other: &Path| -> bool {
                let Ok(other) = normalized(other) else {
                    return false;
                };
                if kind == FileType::Source {
                    dest != other
                } else {
                    dest.parent() != other.parent() || dest.file_stem() != other.file_stem()
                }
            };
            if self.launch.as_ref().is_some_and(|p| differs(p)) {
                warnings
                    .push("W203 WARNING: The output name differs from the file opened at launch");
            } else if kind != FileType::Source && self.current.as_ref().is_some_and(|p| differs(p))
            {
                warnings.push("W204 WARNING: The filename does not match the source file");
            }
        }
        Ok((path, warnings))
    }
    pub fn saved(&mut self, kind: FileType, path: &Path) -> io::Result<()> {
        let dest = normalized(path)?;
        if kind == FileType::Source {
            let previous = self.current.as_ref().and_then(|p| normalized(p).ok());
            if previous.as_ref() != Some(&dest) {
                self.acknowledged = Default::default();
            }
            self.current = Some(path.to_path_buf());
        }
        self.acknowledged[kind.index()] = Some(dest);
        Ok(())
    }
    pub fn complete(&self, text: &str) -> Vec<(String, bool)> {
        let split = text.rfind('/').map_or(0, |i| i + 1);
        let (dir_text, prefix) = text.split_at(split);
        let dir = if dir_text.is_empty() {
            self.cwd.clone()
        } else {
            absolute(&self.cwd, Path::new(dir_text))
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut found: Vec<_> = entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                if !name.starts_with(prefix) {
                    return None;
                }
                let is_dir = e.path().is_dir();
                Some((
                    format!("{dir_text}{name}{}", if is_dir { "/" } else { "" }),
                    is_dir,
                ))
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }
}

fn generated_stem(source: &str) -> String {
    let table: toml::Table = source
        .parse()
        .expect("invalid embedded filename word lists");
    let words = |key: &str| -> Vec<&str> {
        table
            .get(key)
            .and_then(toml::Value::as_array)
            .expect("filename word list must be an array")
            .iter()
            .map(|v| v.as_str().expect("filename words must be strings"))
            .collect()
    };
    let first = words("first");
    let second = words("second");
    if first.is_empty() || second.is_empty() {
        "equation".to_owned()
    } else {
        format!(
            "{}-{}",
            first[fastrand::usize(..first.len())],
            second[fastrand::usize(..second.len())]
        )
    }
}
fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}
fn normalized(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }
    Ok(path
        .parent()
        .ok_or_else(|| io::Error::other("missing parent"))?
        .canonicalize()?
        .join(
            path.file_name()
                .ok_or_else(|| io::Error::other("missing filename"))?,
        ))
}

pub fn common_prefix(values: &[(String, bool)]) -> String {
    let Some((first, _)) = values.first() else {
        return String::new();
    };
    let mut end = first.len();
    for (s, _) in values.iter().skip(1) {
        end = first[..end]
            .char_indices()
            .zip(s.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((i, c), _)| i + c.len_utf8());
    }
    first[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_stem_uses_both_lists_and_falls_back_for_empty_lists() {
        assert_eq!(
            generated_stem("first = ['silver']\nsecond = ['fox']"),
            "silver-fox"
        );
        assert_eq!(generated_stem("first = []\nsecond = ['fox']"), "equation");
        assert_eq!(
            generated_stem("first = ['silver']\nsecond = []"),
            "equation"
        );
        let embedded = include_str!("../assets/names.toml");
        let table: toml::Table = embedded.parse().unwrap();
        for key in ["first", "second"] {
            assert!(table[key]
                .as_array()
                .unwrap()
                .iter()
                .all(|v| v.as_str().is_some()));
        }
    }
    #[test]
    fn unnamed_session_reuses_stem_for_source_and_exports() {
        let cwd = std::env::temp_dir();
        let files = Files::new(cwd, None);
        let source = files.default_path(FileType::Source);
        let png = files.default_path(FileType::Png);
        let svg = files.default_path(FileType::Svg);
        let stem = Path::new(&source).file_stem().unwrap();
        assert_eq!(Some(stem), Path::new(&png).file_stem());
        assert_eq!(Some(stem), Path::new(&svg).file_stem());
        assert_eq!(source, files.default_path(FileType::Source));
    }
    #[test]
    fn common_prefix_handles_unicode() {
        assert_eq!(
            common_prefix(&[("éclair".into(), false), ("école".into(), false)]),
            "éc"
        );
    }
    #[test]
    fn names_warnings_follow_acknowledgment_period() {
        let dir = std::env::temp_dir().join(format!("rateq-files-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut files = Files::new(dir.clone(), Some("original.txt".into()));
        let changed = dir.join("changed.txt");
        assert!(files
            .validate(changed.to_str().unwrap(), FileType::Source)
            .unwrap()
            .1
            .iter()
            .any(|w| w.contains("W203")));
        fs::write(&changed, "x").unwrap();
        files.saved(FileType::Source, &changed).unwrap();
        assert!(!files
            .validate(changed.to_str().unwrap(), FileType::Source)
            .unwrap()
            .1
            .iter()
            .any(|w| w.contains("W203")));
        let export = dir.join("changed.png");
        assert!(files
            .validate(export.to_str().unwrap(), FileType::Png)
            .unwrap()
            .1
            .iter()
            .any(|w| w.contains("W203")));
        fs::write(&export, "x").unwrap();
        files.saved(FileType::Png, &export).unwrap();
        assert!(!files
            .validate(export.to_str().unwrap(), FileType::Png)
            .unwrap()
            .1
            .iter()
            .any(|w| w.contains("W203")));
        assert!(files
            .validate(dir.join("changed.svg").to_str().unwrap(), FileType::Svg)
            .unwrap()
            .1
            .iter()
            .any(|w| w.contains("W203")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn validates_destinations_and_completes_paths() {
        let dir =
            std::env::temp_dir().join(format!("rateq-file-validation-{}", std::process::id()));
        fs::create_dir_all(dir.join("child")).unwrap();
        fs::write(dir.join("alpha one.txt"), "x").unwrap();
        fs::write(dir.join("alpha two.txt"), "x").unwrap();
        let files = Files::new(dir.clone(), None);
        assert!(files
            .validate("", FileType::Source)
            .unwrap_err()
            .contains("E203"));
        assert!(files
            .validate("child/", FileType::Source)
            .unwrap_err()
            .contains("E203"));
        assert!(files
            .validate("out.svg", FileType::Png)
            .unwrap_err()
            .contains("E202"));
        assert!(files
            .validate("missing/out.png", FileType::Png)
            .unwrap_err()
            .contains("E204"));
        let (_, warnings) = files.validate("alpha one.txt", FileType::Source).unwrap();
        assert!(warnings.iter().any(|w| w.contains("W201")));
        assert!(warnings.iter().any(|w| w.contains("W202")));
        assert_eq!(common_prefix(&files.complete("alpha")), "alpha ");
        assert_eq!(common_prefix(&files.complete("chi")), "child/");
        fs::remove_dir_all(dir).unwrap();
    }
}
