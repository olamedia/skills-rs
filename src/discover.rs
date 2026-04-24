use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::skill::{parse_skill_md, Skill};

const SKIP_DIRS: &[&str] = &["node_modules", ".git", "dist", "build", "__pycache__"];

const PRIORITY_SUBDIRS: &[&str] = &[
    "skills",
    "skills/.curated",
    "skills/.experimental",
    "skills/.system",
    ".agents/skills",
    ".claude/skills",
    ".cursor/skills",
    ".cline/skills",
    ".codebuddy/skills",
    ".codex/skills",
    ".commandcode/skills",
    ".continue/skills",
    ".goose/skills",
    ".hermes/skills",
    ".iflow/skills",
    ".junie/skills",
    ".kilocode/skills",
    ".kiro/skills",
    ".mux/skills",
    ".neovate/skills",
    ".opencode/skills",
    ".openhands/skills",
    ".pi/skills",
    ".qoder/skills",
    ".roo/skills",
    ".trae/skills",
    ".windsurf/skills",
    ".zencoder/skills",
];

pub fn discover_skills(base_path: &Path, subpath: Option<&str>) -> Vec<Skill> {
    let search_path = match subpath {
        Some(sub) => base_path.join(sub),
        None => base_path.to_path_buf(),
    };

    let mut skills = Vec::new();
    let mut seen_names = HashSet::new();

    // 1. Check root SKILL.md
    if let Some(skill) = try_parse_skill_at(&search_path) {
        seen_names.insert(skill.name.clone());
        skills.push(skill);
        return skills;
    }

    // 2. Search priority subdirectories
    for subdir in PRIORITY_SUBDIRS {
        let dir = search_path.join(subdir);
        scan_dir_for_skills(&dir, &mut skills, &mut seen_names);
    }

    // 3. Recursive fallback if nothing found
    if skills.is_empty() {
        find_skills_recursive(&search_path, 0, 5, &mut skills, &mut seen_names);
    }

    skills
}

fn try_parse_skill_at(dir: &Path) -> Option<Skill> {
    let skill_md = dir.join("SKILL.md");
    if skill_md.is_file() {
        parse_skill_md(&skill_md)
    } else {
        None
    }
}

fn scan_dir_for_skills(dir: &Path, skills: &mut Vec<Skill>, seen: &mut HashSet<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let child = entry.path();
        if let Some(skill) = try_parse_skill_at(&child) {
            if seen.insert(skill.name.clone()) {
                skills.push(skill);
            }
        }
    }
}

fn find_skills_recursive(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    skills: &mut Vec<Skill>,
    seen: &mut HashSet<String>,
) {
    if depth > max_depth {
        return;
    }

    if let Some(skill) = try_parse_skill_at(dir) {
        if seen.insert(skill.name.clone()) {
            skills.push(skill);
        }
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if SKIP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }
        find_skills_recursive(&entry.path(), depth + 1, max_depth, skills, seen);
    }
}

pub fn filter_skills_by_name<'a>(skills: &'a [Skill], names: &[String]) -> Vec<&'a Skill> {
    let lower_names: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    skills
        .iter()
        .filter(|s| lower_names.iter().any(|n| n == &s.name.to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_skill(dir: &Path, name: &str, desc: &str) {
        fs::create_dir_all(dir).unwrap();
        let mut f = fs::File::create(dir.join("SKILL.md")).unwrap();
        writeln!(f, "---\nname: {name}\ndescription: {desc}\n---\n# {name}").unwrap();
    }

    #[test]
    fn test_discover_root_skill() {
        let tmp = tempfile::tempdir().unwrap();
        make_skill(tmp.path(), "root-skill", "At root");
        let found = discover_skills(tmp.path(), None);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "root-skill");
    }

    #[test]
    fn test_discover_in_skills_dir() {
        let tmp = tempfile::tempdir().unwrap();
        make_skill(&tmp.path().join("skills/alpha"), "alpha", "Alpha skill");
        make_skill(&tmp.path().join("skills/beta"), "beta", "Beta skill");
        let found = discover_skills(tmp.path(), None);
        assert_eq!(found.len(), 2);
        let names: HashSet<_> = found.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains("alpha"));
        assert!(names.contains("beta"));
    }

    #[test]
    fn test_discover_dedup() {
        let tmp = tempfile::tempdir().unwrap();
        make_skill(&tmp.path().join("skills/dup"), "dup", "First");
        make_skill(&tmp.path().join(".claude/skills/dup"), "dup", "Second");
        let found = discover_skills(tmp.path(), None);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_discover_recursive_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        make_skill(&tmp.path().join("deep/nested/skill"), "deep-skill", "Deep");
        let found = discover_skills(tmp.path(), None);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "deep-skill");
    }

    #[test]
    fn test_discover_with_subpath() {
        let tmp = tempfile::tempdir().unwrap();
        make_skill(&tmp.path().join("sub/skills/s1"), "s1", "Sub skill");
        make_skill(&tmp.path().join("skills/s2"), "s2", "Root skill");
        let found = discover_skills(tmp.path(), Some("sub"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "s1");
    }

    #[test]
    fn test_discover_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let found = discover_skills(tmp.path(), None);
        assert!(found.is_empty());
    }

    #[test]
    fn test_filter_skills_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        make_skill(&tmp.path().join("skills/a"), "alpha", "A");
        make_skill(&tmp.path().join("skills/b"), "beta", "B");
        let found = discover_skills(tmp.path(), None);
        let filtered = filter_skills_by_name(&found, &["Alpha".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "alpha");
    }
}
