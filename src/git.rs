use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const CLONE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum GitError {
    GitNotFound,
    Timeout { url: String },
    AuthFailed { url: String },
    CloneFailed { url: String, message: String },
    Io(io::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::GitNotFound => write!(
                f,
                "git is not installed or not on PATH. Install git to use remote sources."
            ),
            GitError::Timeout { url } => write!(
                f,
                "Clone timed out after 60s for {url}.\n  \
                 Check network access and credentials (SSH keys, git auth)."
            ),
            GitError::AuthFailed { url } => write!(
                f,
                "Authentication failed for {url}.\n  \
                 For SSH: check keys with `ssh -T git@github.com`\n  \
                 For HTTPS: run `gh auth login` or configure git credentials"
            ),
            GitError::CloneFailed { url, message } => {
                write!(f, "Failed to clone {url}: {message}")
            }
            GitError::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl From<io::Error> for GitError {
    fn from(e: io::Error) -> Self {
        GitError::Io(e)
    }
}

pub fn check_git_available() -> Result<(), GitError> {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|_| GitError::GitNotFound)?;
    Ok(())
}

/// Clone a repo to a temporary directory. Returns the path.
/// The caller is responsible for cleaning up the directory.
pub fn clone_repo(url: &str, git_ref: Option<&str>) -> Result<PathBuf, GitError> {
    check_git_available()?;

    let temp_dir = tempfile::Builder::new()
        .prefix("skills-")
        .tempdir()
        .map_err(GitError::Io)?;
    let dest = temp_dir.keep();

    let mut cmd = Command::new("git");
    cmd.arg("clone")
        .arg("--depth")
        .arg("1");

    if let Some(r) = git_ref {
        cmd.arg("--branch").arg(r);
    }

    cmd.arg(url)
        .arg(&dest)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LFS_SKIP_SMUDGE", "1");

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                GitError::GitNotFound
            } else {
                GitError::Io(e)
            }
        })?;

    let result = wait_with_timeout(&mut child, CLONE_TIMEOUT);

    match result {
        Ok(output) => {
            if output.success {
                Ok(dest)
            } else {
                // Cleanup on failure
                let _ = std::fs::remove_dir_all(&dest);
                let stderr = output.stderr;
                if is_auth_error(&stderr) {
                    Err(GitError::AuthFailed {
                        url: url.to_string(),
                    })
                } else {
                    Err(GitError::CloneFailed {
                        url: url.to_string(),
                        message: stderr,
                    })
                }
            }
        }
        Err(_) => {
            let _ = child.kill();
            let _ = std::fs::remove_dir_all(&dest);
            Err(GitError::Timeout {
                url: url.to_string(),
            })
        }
    }
}

pub fn cleanup_clone_dir(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

struct WaitOutput {
    success: bool,
    stderr: String,
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<WaitOutput, ()> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                return Ok(WaitOutput {
                    success: status.success(),
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    return Err(());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return Err(()),
        }
    }
}

fn is_auth_error(stderr: &str) -> bool {
    stderr.contains("Authentication failed")
        || stderr.contains("could not read Username")
        || stderr.contains("Permission denied")
        || stderr.contains("Repository not found")
}
