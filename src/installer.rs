use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::agents::{agent_skills_dir, canonical_skills_dir, AgentConfig};
use crate::sanitize::{is_path_safe, sanitize_name};
use crate::skill::Skill;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstallMode {
    Symlink,
    Copy,
}

#[derive(Debug)]
pub struct InstallResult {
    pub skill_name: String,
    pub agent_name: String,
    pub dest: PathBuf,
    #[allow(dead_code)]
    pub mode: InstallMode,
}

#[derive(Debug)]
pub enum InstallError {
    UnsafePath { path: PathBuf },
    Io(io::Error),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::UnsafePath { path } => {
                write!(f, "Path traversal blocked: {}", path.display())
            }
            InstallError::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl From<io::Error> for InstallError {
    fn from(e: io::Error) -> Self {
        InstallError::Io(e)
    }
}

/// Install a skill to the canonical location, then link/copy to each agent dir.
///
/// Flow:
/// 1. Copy source dir -> canonical_dir/<sanitized_name>
/// 2. For each agent: symlink (or copy) from canonical -> agent's skills dir
pub fn install_skill(
    skill: &Skill,
    agents: &[&AgentConfig],
    mode: InstallMode,
    global: bool,
    home: &Path,
    cwd: &Path,
) -> Result<Vec<InstallResult>, InstallError> {
    let safe_name = sanitize_name(&skill.name);
    let canonical_base = canonical_skills_dir(global, home, cwd);
    let canonical_dest = canonical_base.join(&safe_name);

    // Safety: don't write outside canonical base
    if !is_path_safe(&canonical_base, &canonical_dest) {
        return Err(InstallError::UnsafePath {
            path: canonical_dest,
        });
    }

    // 1. Copy source to canonical location
    if canonical_dest.exists() {
        fs::remove_dir_all(&canonical_dest)?;
    }
    copy_dir_recursive(&skill.path, &canonical_dest)?;

    // 2. Link/copy to each agent's skills directory
    let mut results = Vec::new();

    for agent in agents {
        let agent_dir = agent_skills_dir(agent, global, home, cwd);
        let agent_skill_dest = agent_dir.join(&safe_name);

        // Skip if agent dir == canonical dir (already handled)
        if agent_skill_dest == canonical_dest {
            results.push(InstallResult {
                skill_name: safe_name.clone(),
                agent_name: agent.name.to_string(),
                dest: canonical_dest.clone(),
                mode: InstallMode::Copy,
            });
            continue;
        }

        fs::create_dir_all(&agent_dir)?;

        if agent_skill_dest.exists() || agent_skill_dest.symlink_metadata().is_ok() {
            if agent_skill_dest.is_dir() {
                fs::remove_dir_all(&agent_skill_dest)?;
            } else {
                fs::remove_file(&agent_skill_dest)?;
            }
        }

        match mode {
            InstallMode::Symlink => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&canonical_dest, &agent_skill_dest)?;
                #[cfg(windows)]
                std::os::windows::fs::symlink_dir(&canonical_dest, &agent_skill_dest)?;
            }
            InstallMode::Copy => {
                copy_dir_recursive(&canonical_dest, &agent_skill_dest)?;
            }
        }

        results.push(InstallResult {
            skill_name: safe_name.clone(),
            agent_name: agent.name.to_string(),
            dest: agent_skill_dest,
            mode,
        });
    }

    Ok(results)
}

/// Remove a skill from an agent's directory.
pub fn remove_skill(
    name: &str,
    agent: &AgentConfig,
    global: bool,
    home: &Path,
    cwd: &Path,
) -> Result<PathBuf, InstallError> {
    let safe_name = sanitize_name(name);
    let agent_dir = agent_skills_dir(agent, global, home, cwd);
    let target = agent_dir.join(&safe_name);

    if target.symlink_metadata().is_ok() {
        if target.is_dir() && !target.symlink_metadata()?.file_type().is_symlink() {
            fs::remove_dir_all(&target)?;
        } else {
            fs::remove_file(&target)?;
        }
    }

    Ok(target)
}

/// Remove a skill from the canonical location.
pub fn remove_canonical(
    name: &str,
    global: bool,
    home: &Path,
    cwd: &Path,
) -> Result<PathBuf, InstallError> {
    let safe_name = sanitize_name(name);
    let canonical_base = canonical_skills_dir(global, home, cwd);
    let target = canonical_base.join(&safe_name);

    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    Ok(target)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::find_agent;
    use std::io::Write;

    fn make_skill(dir: &Path, name: &str) -> Skill {
        fs::create_dir_all(dir).unwrap();
        let mut f = fs::File::create(dir.join("SKILL.md")).unwrap();
        writeln!(f, "---\nname: {name}\ndescription: test\n---\n# {name}").unwrap();
        Skill {
            name: name.to_string(),
            description: "test".to_string(),
            path: dir.to_path_buf(),
            is_internal: false,
        }
    }

    #[test]
    fn test_install_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("source/my-skill");
        let skill = make_skill(&src_dir, "my-skill");
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let agent = find_agent("claude-code").unwrap();
        let results =
            install_skill(&skill, &[agent], InstallMode::Copy, false, &home, &cwd).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill_name, "my-skill");
        assert!(results[0].dest.join("SKILL.md").exists());
    }

    #[test]
    fn test_install_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("source/my-skill");
        let skill = make_skill(&src_dir, "my-skill");
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let agent = find_agent("claude-code").unwrap();
        let results =
            install_skill(&skill, &[agent], InstallMode::Symlink, false, &home, &cwd).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].dest.symlink_metadata().unwrap().file_type().is_symlink());
    }

    #[test]
    fn test_install_universal_agent_skips_extra_link() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("source/my-skill");
        let skill = make_skill(&src_dir, "my-skill");
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let agent = find_agent("cursor").unwrap();
        let results =
            install_skill(&skill, &[agent], InstallMode::Symlink, false, &home, &cwd).unwrap();

        // Universal agents use canonical dir directly
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].mode, InstallMode::Copy);
    }

    #[test]
    fn test_remove_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("source/my-skill");
        let skill = make_skill(&src_dir, "my-skill");
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let agent = find_agent("claude-code").unwrap();
        install_skill(&skill, &[agent], InstallMode::Copy, false, &home, &cwd).unwrap();

        let removed = remove_skill("my-skill", agent, false, &home, &cwd).unwrap();
        assert!(!removed.exists());
    }

    #[test]
    fn test_remove_canonical() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("source/my-skill");
        let skill = make_skill(&src_dir, "my-skill");
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let agent = find_agent("cursor").unwrap();
        install_skill(&skill, &[agent], InstallMode::Copy, false, &home, &cwd).unwrap();

        let removed = remove_canonical("my-skill", false, &home, &cwd).unwrap();
        assert!(!removed.exists());
    }

    #[test]
    fn test_reinstall_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("source/my-skill");
        let skill = make_skill(&src_dir, "my-skill");
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let agent = find_agent("claude-code").unwrap();
        install_skill(&skill, &[agent], InstallMode::Copy, false, &home, &cwd).unwrap();
        let results =
            install_skill(&skill, &[agent], InstallMode::Copy, false, &home, &cwd).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].dest.join("SKILL.md").exists());
    }
}
