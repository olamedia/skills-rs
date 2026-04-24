use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum SourceType {
    Local,
    GitHub,
    GitLab,
    Git,
}

#[derive(Debug, Clone)]
pub struct ParsedSource {
    pub source_type: SourceType,
    pub url: String,
    pub local_path: Option<PathBuf>,
    pub git_ref: Option<String>,
    pub subpath: Option<String>,
    pub skill_filter: Option<String>,
}

fn is_local_path(input: &str) -> bool {
    Path::new(input).is_absolute()
        || input.starts_with("./")
        || input.starts_with("../")
        || input == "."
        || input == ".."
}

fn parse_fragment(input: &str) -> (&str, Option<String>, Option<String>) {
    if let Some(hash_pos) = input.find('#') {
        let base = &input[..hash_pos];
        let fragment = &input[hash_pos + 1..];
        if fragment.is_empty() {
            return (input, None, None);
        }
        if let Some(at_pos) = fragment.find('@') {
            let git_ref = &fragment[..at_pos];
            let skill = &fragment[at_pos + 1..];
            return (
                base,
                if git_ref.is_empty() { None } else { Some(git_ref.to_string()) },
                if skill.is_empty() { None } else { Some(skill.to_string()) },
            );
        }
        (base, Some(fragment.to_string()), None)
    } else {
        (input, None, None)
    }
}

pub fn parse_source(input: &str) -> ParsedSource {
    if is_local_path(input) {
        let resolved = std::env::current_dir()
            .map(|cwd| cwd.join(input))
            .unwrap_or_else(|_| PathBuf::from(input));
        return ParsedSource {
            source_type: SourceType::Local,
            url: resolved.display().to_string(),
            local_path: Some(resolved),
            git_ref: None,
            subpath: None,
            skill_filter: None,
        };
    }

    let (input_clean, frag_ref, frag_skill) = parse_fragment(input);

    // gitlab: prefix
    if let Some(rest) = input_clean.strip_prefix("gitlab:") {
        return ParsedSource {
            source_type: SourceType::GitLab,
            url: format!("https://gitlab.com/{}.git", rest),
            local_path: None,
            git_ref: frag_ref,
            subpath: None,
            skill_filter: frag_skill,
        };
    }

    // github: prefix
    if let Some(rest) = input_clean.strip_prefix("github:") {
        return ParsedSource {
            source_type: SourceType::GitHub,
            url: format!("https://github.com/{}.git", rest),
            local_path: None,
            git_ref: frag_ref,
            subpath: None,
            skill_filter: frag_skill,
        };
    }

    // GitHub tree URL with subpath: github.com/owner/repo/tree/branch/path
    if let Some(caps) = regex_github_tree_path(input_clean) {
        return ParsedSource {
            source_type: SourceType::GitHub,
            url: format!("https://github.com/{}/{}.git", caps.0, caps.1),
            local_path: None,
            git_ref: Some(caps.2),
            subpath: Some(caps.3),
            skill_filter: frag_skill,
        };
    }

    // GitHub tree URL branch only: github.com/owner/repo/tree/branch
    if let Some(caps) = regex_github_tree(input_clean) {
        return ParsedSource {
            source_type: SourceType::GitHub,
            url: format!("https://github.com/{}/{}.git", caps.0, caps.1),
            local_path: None,
            git_ref: Some(caps.2),
            subpath: None,
            skill_filter: frag_skill,
        };
    }

    // GitHub URL: github.com/owner/repo
    if let Some(caps) = regex_github_repo(input_clean) {
        return ParsedSource {
            source_type: SourceType::GitHub,
            url: format!("https://github.com/{}/{}.git", caps.0, caps.1),
            local_path: None,
            git_ref: frag_ref,
            subpath: None,
            skill_filter: frag_skill,
        };
    }

    // GitLab tree URL with subpath: /-/tree/branch/path
    if let Some(caps) = regex_gitlab_tree_path(input_clean) {
        return ParsedSource {
            source_type: SourceType::GitLab,
            url: format!("{}://{}/{}.git", caps.0, caps.1, caps.2),
            local_path: None,
            git_ref: Some(caps.3),
            subpath: Some(caps.4),
            skill_filter: frag_skill,
        };
    }

    // GitLab tree URL branch only
    if let Some(caps) = regex_gitlab_tree(input_clean) {
        return ParsedSource {
            source_type: SourceType::GitLab,
            url: format!("{}://{}/{}.git", caps.0, caps.1, caps.2),
            local_path: None,
            git_ref: Some(caps.3),
            subpath: None,
            skill_filter: frag_skill,
        };
    }

    // gitlab.com URL
    if input_clean.contains("gitlab.com/") {
        if let Some(caps) = regex_gitlab_repo(input_clean) {
            return ParsedSource {
                source_type: SourceType::GitLab,
                url: format!("https://gitlab.com/{}.git", caps),
                local_path: None,
                git_ref: frag_ref,
                subpath: None,
                skill_filter: frag_skill,
            };
        }
    }

    // SSH URL: git@host:path.git
    if input_clean.starts_with("git@") {
        return ParsedSource {
            source_type: SourceType::Git,
            url: input_clean.to_string(),
            local_path: None,
            git_ref: frag_ref,
            subpath: None,
            skill_filter: frag_skill,
        };
    }

    // owner/repo@skill shorthand
    if let Some(caps) = regex_at_skill(input_clean) {
        return ParsedSource {
            source_type: SourceType::GitHub,
            url: format!("https://github.com/{}/{}.git", caps.0, caps.1),
            local_path: None,
            git_ref: frag_ref,
            subpath: None,
            skill_filter: Some(frag_skill.unwrap_or(caps.2)),
        };
    }

    // GitHub shorthand: owner/repo or owner/repo/subpath
    if !input_clean.contains(':') && !input_clean.starts_with('.') && !input_clean.starts_with('/') {
        if let Some(caps) = regex_shorthand(input_clean) {
            return ParsedSource {
                source_type: SourceType::GitHub,
                url: format!("https://github.com/{}/{}.git", caps.0, caps.1),
                local_path: None,
                git_ref: frag_ref,
                subpath: caps.2,
                skill_filter: frag_skill,
            };
        }
    }

    // Fallback: treat as git URL
    ParsedSource {
        source_type: SourceType::Git,
        url: input_clean.to_string(),
        local_path: None,
        git_ref: frag_ref,
        subpath: None,
        skill_filter: frag_skill,
    }
}

// --- Regex helpers returning tuple captures (avoid compiled regex for simplicity) ---

fn regex_github_tree_path(input: &str) -> Option<(String, String, String, String)> {
    let re = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)/tree/([^/]+)/(.+)").unwrap();
    re.captures(input).map(|c| {
        (
            c[1].to_string(),
            c[2].to_string(),
            c[3].to_string(),
            c[4].to_string(),
        )
    })
}

fn regex_github_tree(input: &str) -> Option<(String, String, String)> {
    let re = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)/tree/([^/]+)$").unwrap();
    re.captures(input).map(|c| {
        (c[1].to_string(), c[2].to_string(), c[3].to_string())
    })
}

fn regex_github_repo(input: &str) -> Option<(String, String)> {
    let re = regex::Regex::new(r"github\.com/([^/]+)/([^/]+?)(?:\.git)?/?$").unwrap();
    re.captures(input).map(|c| {
        (c[1].to_string(), c[2].to_string())
    })
}

fn regex_gitlab_tree_path(input: &str) -> Option<(String, String, String, String, String)> {
    let re = regex::Regex::new(r"^(https?):\/\/([^/]+)/(.+?)/\-/tree/([^/]+)/(.+)").unwrap();
    re.captures(input).map(|c| {
        (
            c[1].to_string(),
            c[2].to_string(),
            c[3].to_string(),
            c[4].to_string(),
            c[5].to_string(),
        )
    })
}

fn regex_gitlab_tree(input: &str) -> Option<(String, String, String, String)> {
    let re = regex::Regex::new(r"^(https?):\/\/([^/]+)/(.+?)/\-/tree/([^/]+)$").unwrap();
    re.captures(input).map(|c| {
        (
            c[1].to_string(),
            c[2].to_string(),
            c[3].to_string(),
            c[4].to_string(),
        )
    })
}

fn regex_gitlab_repo(input: &str) -> Option<String> {
    let re = regex::Regex::new(r"gitlab\.com/(.+?)(?:\.git)?/?$").unwrap();
    re.captures(input).and_then(|c| {
        let path = c[1].to_string();
        if path.contains('/') { Some(path) } else { None }
    })
}

fn regex_at_skill(input: &str) -> Option<(String, String, String)> {
    let re = regex::Regex::new(r"^([^/]+)/([^/@]+)@(.+)$").unwrap();
    if input.contains(':') || input.starts_with('.') || input.starts_with('/') {
        return None;
    }
    re.captures(input).map(|c| {
        (c[1].to_string(), c[2].to_string(), c[3].to_string())
    })
}

fn regex_shorthand(input: &str) -> Option<(String, String, Option<String>)> {
    let re = regex::Regex::new(r"^([^/]+)/([^/]+)(?:/(.+?))?/?$").unwrap();
    re.captures(input).map(|c| {
        (
            c[1].to_string(),
            c[2].to_string(),
            c.get(3).map(|m| m.as_str().to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_shorthand() {
        let s = parse_source("owner/repo");
        assert_eq!(s.source_type, SourceType::GitHub);
        assert_eq!(s.url, "https://github.com/owner/repo.git");
    }

    #[test]
    fn test_github_url() {
        let s = parse_source("https://github.com/owner/repo");
        assert_eq!(s.source_type, SourceType::GitHub);
        assert_eq!(s.url, "https://github.com/owner/repo.git");
    }

    #[test]
    fn test_github_tree_url() {
        let s = parse_source("https://github.com/owner/repo/tree/main/skills/my-skill");
        assert_eq!(s.source_type, SourceType::GitHub);
        assert_eq!(s.git_ref, Some("main".into()));
        assert_eq!(s.subpath, Some("skills/my-skill".into()));
    }

    #[test]
    fn test_gitlab_prefix() {
        let s = parse_source("gitlab:group/repo");
        assert_eq!(s.source_type, SourceType::GitLab);
        assert_eq!(s.url, "https://gitlab.com/group/repo.git");
    }

    #[test]
    fn test_gitlab_tree_url() {
        let s = parse_source("https://gitlab.com/group/repo/-/tree/main/sub");
        assert_eq!(s.source_type, SourceType::GitLab);
        assert_eq!(s.git_ref, Some("main".into()));
        assert_eq!(s.subpath, Some("sub".into()));
    }

    #[test]
    fn test_ssh_url() {
        let s = parse_source("git@github.com:owner/repo.git");
        assert_eq!(s.source_type, SourceType::Git);
        assert_eq!(s.url, "git@github.com:owner/repo.git");
    }

    #[test]
    fn test_local_relative() {
        let s = parse_source("./my-skills");
        assert_eq!(s.source_type, SourceType::Local);
        assert!(s.local_path.is_some());
    }

    #[test]
    fn test_local_absolute() {
        let s = parse_source("/tmp/skills");
        assert_eq!(s.source_type, SourceType::Local);
        assert_eq!(s.local_path, Some(PathBuf::from("/tmp/skills")));
    }

    #[test]
    fn test_fragment_ref() {
        let s = parse_source("owner/repo#v2");
        assert_eq!(s.source_type, SourceType::GitHub);
        assert_eq!(s.git_ref, Some("v2".into()));
    }

    #[test]
    fn test_at_skill_filter() {
        let s = parse_source("owner/repo@my-skill");
        assert_eq!(s.source_type, SourceType::GitHub);
        assert_eq!(s.skill_filter, Some("my-skill".into()));
    }

    #[test]
    fn test_shorthand_with_subpath() {
        let s = parse_source("owner/repo/skills/my-skill");
        assert_eq!(s.source_type, SourceType::GitHub);
        assert_eq!(s.subpath, Some("skills/my-skill".into()));
    }

    #[test]
    fn test_gitlab_repo_url() {
        let s = parse_source("https://gitlab.com/group/subgroup/repo");
        assert_eq!(s.source_type, SourceType::GitLab);
        assert_eq!(s.url, "https://gitlab.com/group/subgroup/repo.git");
    }
}
