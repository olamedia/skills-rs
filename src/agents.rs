use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: &'static str,
    pub display_name: &'static str,
    pub skills_dir: &'static str,
    pub global_skills_dir_suffix: Option<&'static str>,
    pub detect_dir: &'static str,
    pub is_universal: bool,
}

impl AgentConfig {
    pub fn global_skills_dir(&self, home: &Path) -> Option<PathBuf> {
        self.global_skills_dir_suffix.map(|s| home.join(s))
    }

    pub fn project_skills_dir(&self, cwd: &Path) -> PathBuf {
        cwd.join(self.skills_dir)
    }

    pub fn is_installed(&self, home: &Path) -> bool {
        home.join(self.detect_dir).exists()
    }
}

pub const AGENTS: &[AgentConfig] = &[
    AgentConfig {
        name: "claude-code",
        display_name: "Claude Code",
        skills_dir: ".claude/skills",
        global_skills_dir_suffix: Some(".claude/skills"),
        detect_dir: ".claude",
        is_universal: false,
    },
    AgentConfig {
        name: "cursor",
        display_name: "Cursor",
        skills_dir: ".agents/skills",
        global_skills_dir_suffix: Some(".cursor/skills"),
        detect_dir: ".cursor",
        is_universal: true,
    },
    AgentConfig {
        name: "codex",
        display_name: "Codex",
        skills_dir: ".agents/skills",
        global_skills_dir_suffix: Some(".codex/skills"),
        detect_dir: ".codex",
        is_universal: true,
    },
    AgentConfig {
        name: "opencode",
        display_name: "OpenCode",
        skills_dir: ".agents/skills",
        // Properly XDG: ~/.config/opencode/skills
        global_skills_dir_suffix: Some(".config/opencode/skills"),
        detect_dir: ".config/opencode",
        is_universal: true,
    },
    AgentConfig {
        name: "cline",
        display_name: "Cline",
        skills_dir: ".agents/skills",
        global_skills_dir_suffix: Some(".agents/skills"),
        detect_dir: ".cline",
        is_universal: true,
    },
    AgentConfig {
        name: "windsurf",
        display_name: "Windsurf",
        skills_dir: ".windsurf/skills",
        global_skills_dir_suffix: Some(".codeium/windsurf/skills"),
        detect_dir: ".codeium/windsurf",
        is_universal: false,
    },
    AgentConfig {
        name: "roo",
        display_name: "Roo Code",
        skills_dir: ".roo/skills",
        global_skills_dir_suffix: Some(".roo/skills"),
        detect_dir: ".roo",
        is_universal: false,
    },
    AgentConfig {
        name: "goose",
        display_name: "Goose",
        skills_dir: ".goose/skills",
        // XDG: ~/.config/goose/skills
        global_skills_dir_suffix: Some(".config/goose/skills"),
        detect_dir: ".config/goose",
        is_universal: false,
    },
    AgentConfig {
        name: "augment",
        display_name: "Augment",
        skills_dir: ".augment/skills",
        global_skills_dir_suffix: Some(".augment/skills"),
        detect_dir: ".augment",
        is_universal: false,
    },
    AgentConfig {
        name: "continue",
        display_name: "Continue",
        skills_dir: ".continue/skills",
        global_skills_dir_suffix: Some(".continue/skills"),
        detect_dir: ".continue",
        is_universal: false,
    },
    AgentConfig {
        name: "github-copilot",
        display_name: "GitHub Copilot",
        skills_dir: ".agents/skills",
        global_skills_dir_suffix: Some(".copilot/skills"),
        detect_dir: ".copilot",
        is_universal: true,
    },
    AgentConfig {
        name: "gemini-cli",
        display_name: "Gemini CLI",
        skills_dir: ".agents/skills",
        global_skills_dir_suffix: Some(".gemini/skills"),
        detect_dir: ".gemini",
        is_universal: true,
    },
    AgentConfig {
        name: "hermes",
        display_name: "Hermes",
        skills_dir: ".hermes/skills",
        global_skills_dir_suffix: Some(".hermes/skills"),
        detect_dir: ".hermes",
        is_universal: false,
    },
];

pub const CANONICAL_SKILLS_DIR: &str = ".agents/skills";

pub fn detect_installed_agents(home: &Path) -> Vec<&'static AgentConfig> {
    AGENTS.iter().filter(|a| a.is_installed(home)).collect()
}

pub fn find_agent(name: &str) -> Option<&'static AgentConfig> {
    AGENTS.iter().find(|a| a.name == name)
}

pub fn canonical_skills_dir(global: bool, home: &Path, cwd: &Path) -> PathBuf {
    let base = if global { home } else { cwd };
    base.join(CANONICAL_SKILLS_DIR)
}

pub fn agent_skills_dir(
    agent: &AgentConfig,
    global: bool,
    home: &Path,
    cwd: &Path,
) -> PathBuf {
    if agent.is_universal {
        return canonical_skills_dir(global, home, cwd);
    }
    if global {
        agent.global_skills_dir(home).unwrap_or_else(|| {
            cwd.join(agent.skills_dir)
        })
    } else {
        agent.project_skills_dir(cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_count() {
        assert_eq!(AGENTS.len(), 13);
    }

    #[test]
    fn test_find_agent() {
        let a = find_agent("claude-code").unwrap();
        assert_eq!(a.display_name, "Claude Code");
        assert!(!a.is_universal);
    }

    #[test]
    fn test_find_agent_missing() {
        assert!(find_agent("nonexistent").is_none());
    }

    #[test]
    fn test_universal_agents() {
        let ua = universal_agents();
        let names: Vec<_> = ua.iter().map(|a| a.name).collect();
        assert!(names.contains(&"cursor"));
        assert!(names.contains(&"codex"));
        assert!(!names.contains(&"claude-code"));
    }

    #[test]
    fn test_canonical_dir() {
        let home = Path::new("/home/user");
        let cwd = Path::new("/project");
        assert_eq!(
            canonical_skills_dir(true, home, cwd),
            PathBuf::from("/home/user/.agents/skills")
        );
        assert_eq!(
            canonical_skills_dir(false, home, cwd),
            PathBuf::from("/project/.agents/skills")
        );
    }

    #[test]
    fn test_agent_skills_dir_universal() {
        let a = find_agent("cursor").unwrap();
        let home = Path::new("/home/user");
        let cwd = Path::new("/project");
        assert_eq!(
            agent_skills_dir(a, true, home, cwd),
            PathBuf::from("/home/user/.agents/skills")
        );
        assert_eq!(
            agent_skills_dir(a, false, home, cwd),
            PathBuf::from("/project/.agents/skills")
        );
    }

    #[test]
    fn test_agent_skills_dir_non_universal() {
        let a = find_agent("claude-code").unwrap();
        let home = Path::new("/home/user");
        let cwd = Path::new("/project");
        assert_eq!(
            agent_skills_dir(a, true, home, cwd),
            PathBuf::from("/home/user/.claude/skills")
        );
        assert_eq!(
            agent_skills_dir(a, false, home, cwd),
            PathBuf::from("/project/.claude/skills")
        );
    }

    #[test]
    fn test_detect_empty_home() {
        let dir = tempfile::tempdir().unwrap();
        let detected = detect_installed_agents(dir.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn test_detect_with_cursor() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".cursor")).unwrap();
        let detected = detect_installed_agents(dir.path());
        let names: Vec<_> = detected.iter().map(|a| a.name).collect();
        assert!(names.contains(&"cursor"));
    }
}
