use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub is_internal: bool,
}

#[derive(Deserialize)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    metadata: Option<Metadata>,
}

#[derive(Deserialize)]
struct Metadata {
    internal: Option<bool>,
}

pub fn parse_skill_md(skill_md_path: &Path) -> Option<Skill> {
    let content = fs::read_to_string(skill_md_path).ok()?;
    let fm = extract_frontmatter(&content)?;

    let name = fm.name.filter(|n| !n.is_empty())?;
    let description = fm.description.filter(|d| !d.is_empty())?;
    let is_internal = fm.metadata.and_then(|m| m.internal).unwrap_or(false);

    Some(Skill {
        name,
        description,
        path: skill_md_path.parent()?.to_path_buf(),
        is_internal,
    })
}

fn extract_frontmatter(content: &str) -> Option<Frontmatter> {
    let re = regex::Regex::new(r"(?s)^---\r?\n(.*?)\r?\n---").unwrap();
    let caps = re.captures(content)?;
    let yaml_str = caps.get(1)?.as_str();
    serde_yaml::from_str(yaml_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_skill(dir: &Path, content: &str) -> PathBuf {
        let skill_path = dir.join("SKILL.md");
        let mut f = fs::File::create(&skill_path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        skill_path
    }

    #[test]
    fn test_valid_skill() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_skill(
            dir.path(),
            "---\nname: test-skill\ndescription: A test skill\n---\n# Test\n",
        );
        let skill = parse_skill_md(&path).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test skill");
        assert!(!skill.is_internal);
        assert_eq!(skill.path, dir.path());
    }

    #[test]
    fn test_missing_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_skill(dir.path(), "---\ndescription: No name\n---\n# Test\n");
        assert!(parse_skill_md(&path).is_none());
    }

    #[test]
    fn test_missing_description() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_skill(dir.path(), "---\nname: test\n---\n# Test\n");
        assert!(parse_skill_md(&path).is_none());
    }

    #[test]
    fn test_internal_skill() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_skill(
            dir.path(),
            "---\nname: internal\ndescription: Hidden\nmetadata:\n  internal: true\n---\n",
        );
        let skill = parse_skill_md(&path).unwrap();
        assert!(skill.is_internal);
    }

    #[test]
    fn test_no_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_skill(dir.path(), "# Just markdown\nNo frontmatter here.\n");
        assert!(parse_skill_md(&path).is_none());
    }

    #[test]
    fn test_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_skill(
            dir.path(),
            "---\nname: \"\"\ndescription: Has desc\n---\n# Test\n",
        );
        assert!(parse_skill_md(&path).is_none());
    }
}
