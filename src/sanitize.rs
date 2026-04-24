use std::path::{Component, Path};

pub fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive hyphens
    let mut result = String::with_capacity(sanitized.len());
    let mut prev_hyphen = false;
    for c in sanitized.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push(c);
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    let result = result.trim_matches(|c: char| c == '.' || c == '-');
    let result = if result.is_empty() {
        "unnamed-skill"
    } else {
        result
    };

    result[..result.len().min(255)].to_string()
}

pub fn is_path_safe(base: &Path, target: &Path) -> bool {
    let Ok(base_canon) = dunce_normalize(base) else {
        return false;
    };
    let Ok(target_canon) = dunce_normalize(target) else {
        return false;
    };
    target_canon.starts_with(&base_canon)
}

/// Normalize a path without requiring it to exist (unlike fs::canonicalize).
fn dunce_normalize(path: &Path) -> Result<std::path::PathBuf, ()> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if components.last() == Some(&Component::Normal(std::ffi::OsStr::new(""))) {
                    return Err(());
                }
                if !components.is_empty() {
                    components.pop();
                }
            }
            Component::CurDir => {}
            other => components.push(other),
        }
    }
    let mut result = std::path::PathBuf::new();
    for c in components {
        result.push(c.as_os_str());
    }
    if result.as_os_str().is_empty() {
        return Err(());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_basic() {
        assert_eq!(sanitize_name("My Skill!"), "my-skill");
        assert_eq!(sanitize_name("hello-world"), "hello-world");
        assert_eq!(sanitize_name("UPPER_case"), "upper_case");
    }

    #[test]
    fn test_sanitize_traversal() {
        assert_eq!(sanitize_name("../hack"), "hack");
        assert_eq!(sanitize_name("...dots..."), "dots");
        assert_eq!(sanitize_name("--dashes--"), "dashes");
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize_name(""), "unnamed-skill");
        assert_eq!(sanitize_name("---"), "unnamed-skill");
        assert_eq!(sanitize_name("..."), "unnamed-skill");
    }

    #[test]
    fn test_sanitize_preserves_dots_underscores() {
        assert_eq!(sanitize_name("v1.0_beta"), "v1.0_beta");
    }

    #[test]
    fn test_path_safe() {
        assert!(is_path_safe(Path::new("/base"), Path::new("/base/sub")));
        assert!(is_path_safe(Path::new("/base"), Path::new("/base")));
        assert!(!is_path_safe(Path::new("/base"), Path::new("/other")));
        assert!(!is_path_safe(Path::new("/base"), Path::new("/base/../other")));
    }
}
