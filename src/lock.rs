use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ---- Global lock: ~/.agents/.skill-lock.json ----

const GLOBAL_LOCK_VERSION: u32 = 3;
const GLOBAL_LOCK_FILE: &str = ".agents/.skill-lock.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_install_to_home: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_install_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_selected_agents: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalLock {
    pub version: u32,
    pub skills: HashMap<String, GlobalLockEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Preferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalLockEntry {
    pub source_url: String,
    pub skill_name: String,
    pub installed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_folder_hash: Option<String>,
    pub agents: Vec<String>,
}

impl GlobalLock {
    pub fn load(home: &Path) -> Self {
        let path = home.join(GLOBAL_LOCK_FILE);
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::empty()),
            Err(_) => Self::empty(),
        }
    }

    pub fn save(&self, home: &Path) -> io::Result<()> {
        let path = home.join(GLOBAL_LOCK_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)
    }

    pub fn empty() -> Self {
        Self {
            version: GLOBAL_LOCK_VERSION,
            skills: HashMap::new(),
            preferences: None,
        }
    }

    pub fn upsert(&mut self, key: &str, entry: GlobalLockEntry) {
        self.skills.insert(key.to_string(), entry);
    }

    pub fn remove(&mut self, key: &str) -> bool {
        self.skills.remove(key).is_some()
    }

    pub fn load_preferences(&self) -> Preferences {
        self.preferences.clone().unwrap_or_default()
    }

    pub fn save_preferences(&mut self, prefs: Preferences) {
        self.preferences = Some(prefs);
    }
}

// ---- Local lock: <project>/.agents/skill-lock.json ----

const LOCAL_LOCK_VERSION: u32 = 1;
const LOCAL_LOCK_FILE: &str = ".agents/skills-lock.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalLock {
    pub version: u32,
    pub skills: HashMap<String, LocalLockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalLockEntry {
    pub source_url: String,
    pub skill_name: String,
    pub installed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed_hash: Option<String>,
    pub agents: Vec<String>,
}

impl LocalLock {
    pub fn load(cwd: &Path) -> Self {
        let path = cwd.join(LOCAL_LOCK_FILE);
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::empty()),
            Err(_) => Self::empty(),
        }
    }

    pub fn save(&self, cwd: &Path) -> io::Result<()> {
        let path = cwd.join(LOCAL_LOCK_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)
    }

    pub fn empty() -> Self {
        Self {
            version: LOCAL_LOCK_VERSION,
            skills: HashMap::new(),
        }
    }

    pub fn upsert(&mut self, key: &str, entry: LocalLockEntry) {
        self.skills.insert(key.to_string(), entry);
    }

    pub fn remove(&mut self, key: &str) -> bool {
        self.skills.remove(key).is_some()
    }
}

// ---- Hashing ----

/// Compute SHA-256 hash of a skill directory contents.
/// Files are sorted alphabetically for deterministic output.
pub fn hash_skill_dir(dir: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut entries = collect_files(dir)?;
    entries.sort();

    for entry in &entries {
        let relative = entry
            .strip_prefix(dir)
            .unwrap_or(entry)
            .to_string_lossy();
        hasher.update(relative.as_bytes());
        let content = fs::read(entry)?;
        hasher.update(&content);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

pub fn now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_global_lock_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = GlobalLock::empty();
        lock.upsert(
            "test-skill",
            GlobalLockEntry {
                source_url: "https://github.com/owner/repo.git".into(),
                skill_name: "test-skill".into(),
                installed_at: "123".into(),
                skill_folder_hash: Some("abc123".into()),
                agents: vec!["cursor".into()],
            },
        );
        lock.save(tmp.path()).unwrap();

        let loaded = GlobalLock::load(tmp.path());
        assert_eq!(loaded.version, 3);
        assert!(loaded.skills.contains_key("test-skill"));
        let entry = &loaded.skills["test-skill"];
        assert_eq!(entry.skill_name, "test-skill");
        assert_eq!(entry.agents, vec!["cursor"]);
    }

    #[test]
    fn test_global_lock_remove() {
        let mut lock = GlobalLock::empty();
        lock.upsert(
            "s1",
            GlobalLockEntry {
                source_url: "x".into(),
                skill_name: "s1".into(),
                installed_at: "0".into(),
                skill_folder_hash: None,
                agents: vec![],
            },
        );
        assert!(lock.remove("s1"));
        assert!(!lock.remove("s1"));
    }

    #[test]
    fn test_local_lock_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = LocalLock::empty();
        lock.upsert(
            "my-skill",
            LocalLockEntry {
                source_url: "./local".into(),
                skill_name: "my-skill".into(),
                installed_at: "456".into(),
                computed_hash: Some("deadbeef".into()),
                agents: vec!["claude-code".into()],
            },
        );
        lock.save(tmp.path()).unwrap();

        let loaded = LocalLock::load(tmp.path());
        assert_eq!(loaded.version, 1);
        assert!(loaded.skills.contains_key("my-skill"));
    }

    #[test]
    fn test_load_missing_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let global = GlobalLock::load(tmp.path());
        assert_eq!(global.skills.len(), 0);

        let local = LocalLock::load(tmp.path());
        assert_eq!(local.skills.len(), 0);
    }

    #[test]
    fn test_hash_skill_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---\nname: test\ndescription: test\n---").unwrap();

        let hash1 = hash_skill_dir(&skill_dir).unwrap();
        assert!(!hash1.is_empty());

        // Same content = same hash
        let hash2 = hash_skill_dir(&skill_dir).unwrap();
        assert_eq!(hash1, hash2);

        // Modified content = different hash
        let mut f2 = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f2, "---\nname: changed\ndescription: changed\n---").unwrap();
        let hash3 = hash_skill_dir(&skill_dir).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let hash = hash_skill_dir(tmp.path()).unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_preferences_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = GlobalLock::empty();
        lock.save_preferences(Preferences {
            last_install_to_home: Some(true),
            last_install_mode: Some("symlink".into()),
            last_selected_agents: Some(vec!["cursor".into(), "claude-code".into()]),
        });
        lock.save(tmp.path()).unwrap();

        let loaded = GlobalLock::load(tmp.path());
        let prefs = loaded.load_preferences();
        assert_eq!(prefs.last_install_to_home, Some(true));
        assert_eq!(prefs.last_install_mode.as_deref(), Some("symlink"));
        assert_eq!(
            prefs.last_selected_agents,
            Some(vec!["cursor".into(), "claude-code".into()])
        );
    }

    #[test]
    fn test_preferences_missing_returns_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let lock = GlobalLock::empty();
        lock.save(tmp.path()).unwrap();

        let loaded = GlobalLock::load(tmp.path());
        let prefs = loaded.load_preferences();
        assert_eq!(prefs.last_install_to_home, None);
        assert_eq!(prefs.last_install_mode, None);
        assert_eq!(prefs.last_selected_agents, None);
    }

    #[test]
    fn test_old_lock_without_preferences_field() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(GLOBAL_LOCK_FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"version":3,"skills":{}}"#).unwrap();

        let loaded = GlobalLock::load(tmp.path());
        assert_eq!(loaded.version, 3);
        assert!(loaded.preferences.is_none());
        let prefs = loaded.load_preferences();
        assert_eq!(prefs.last_install_to_home, None);
    }
}
