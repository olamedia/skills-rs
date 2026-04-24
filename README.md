# skills

Install agent skills from any git source.

A standalone CLI that fetches skill definitions (SKILL.md files) from any git repository—GitHub, GitLab, self-hosted, or local paths—and installs them for your AI coding agents.

## Supported agents

Claude Code, Cursor, Codex, OpenCode, Cline, Windsurf, Roo Code, Goose, Augment, Continue, GitHub Copilot, Gemini CLI, Hermes.

## Install

### From GitHub releases (Linux)

```bash
# Debian/Ubuntu
curl -LO https://github.com/olamedia/skills-rs/releases/latest/download/skills_0.2.0_amd64.deb
sudo dpkg -i skills_0.2.0_amd64.deb

# Fedora/RHEL
curl -LO https://github.com/olamedia/skills-rs/releases/latest/download/skills-0.2.0-1.x86_64.rpm
sudo rpm -i skills-0.2.0-1.x86_64.rpm

# Standalone binary (no package manager)
curl -LO https://github.com/olamedia/skills-rs/releases/latest/download/skills-0.2.0-linux-amd64
chmod +x skills-0.2.0-linux-amd64
sudo mv skills-0.2.0-linux-amd64 /usr/local/bin/skills
```

ARM64 builds are also available — replace `amd64`/`x86_64` with `arm64`/`aarch64`.

### From source

```bash
cargo install --path .

# or build a release binary
./build.sh

# build with .deb and .rpm packages
./build.sh --all
```

## Usage

```bash
# Add skills from a GitHub repo
skills add owner/repo

# Add from a full URL
skills add https://github.com/owner/repo

# Add from GitLab
skills add gitlab:group/repo

# Add from a local directory
skills add ./my-skills

# Add a specific skill from a repo
skills add owner/repo@my-skill

# Add from a branch
skills add owner/repo#v2

# Add from a GitHub tree URL with subpath
skills add https://github.com/owner/repo/tree/main/skills/my-skill

# SSH source
skills add git@github.com:owner/repo.git

# Install all skills, skip prompts, copy instead of symlink
skills add owner/repo --all --copy

# Only install to specific agents
skills add owner/repo --agent cursor --agent claude-code

# List available skills without installing
skills add owner/repo --list

# Install globally (user-level, shared across projects)
skills add owner/repo --global
```

### List installed skills

```bash
skills list
skills list --global
skills list --json
skills list --agent cursor
```

### Remove skills

```bash
skills remove my-skill
skills remove --all --yes
skills remove my-skill --agent cursor
```

### Update skills

```bash
# Update all installed skills to latest
skills update

# Update specific skills
skills update my-skill

# Update only global or only project skills
skills update --global
skills update --project
```

### Create a new skill

```bash
skills init my-skill
# Creates my-skill/SKILL.md with a template
```

## SKILL.md format

Each skill is a directory containing a `SKILL.md` file with YAML frontmatter:

```markdown
---
name: my-skill
description: What the skill does
---

# My Skill

Instructions for the AI agent...
```

Required frontmatter fields: `name`, `description`.

Optional: `metadata.internal: true` hides the skill from default selection.

## How it works

1. **Parse source** — GitHub shorthand, URLs, SSH, GitLab, local paths
2. **Fetch** — shallow `git clone --depth 1` for remote sources
3. **Discover** — scans priority directories (`skills/`, `.agents/skills/`, agent-specific dirs), falls back to recursive search
4. **Select skills** — interactive multi-select (all unchecked by default) or CLI filters (`--skill`, `--all`, `@skill`)
5. **Select agents** — interactive multi-select with saved preferences from previous runs
6. **Choose scope** — "Install to home?" prompt (global vs project). Saved between sessions.
7. **Choose method** — if home=yes, "Symlink or copy to project?" prompt. Saved between sessions.
8. **Install** — copies to canonical location (`.agents/skills/<name>`), then symlinks (or copies) to each agent's skills directory
9. **Lock** — records source, hash, and agent list in `.agents/skill-lock.json` (project) or `~/.agents/.skill-lock.json` (global)

All interactive choices are persisted in `~/.agents/.skill-lock.json` and used as defaults on the next run. CLI flags (`--global`, `--copy`, `--yes`) bypass the corresponding prompts.

## Project structure

```
src/
├── main.rs        CLI entry point (clap)
├── source.rs      Source URL/path parser
├── sanitize.rs    Name sanitization, path safety
├── skill.rs       SKILL.md frontmatter parser
├── agents.rs      Agent registry (13 agents)
├── discover.rs    Skill discovery in repos
├── git.rs         Git clone with timeout/auth
├── installer.rs   Copy/symlink installer
├── lock.rs        Lock file management + hashing
├── prompt.rs      Interactive TTY prompts
├── cmd_add.rs     add command
├── cmd_list.rs    list command
├── cmd_remove.rs  remove command
├── cmd_update.rs  update command
└── cmd_init.rs    init command
```

## Development

```bash
# Run tests
cargo test

# Build debug
cargo build

# Build release
cargo build --release

# Run directly
cargo run -- add ./path/to/skills --yes --all
```

## Security and privacy

- **No telemetry.** The tool collects nothing and makes no background network requests.
- **No external calls** beyond the `git clone` you explicitly request. No analytics, no registries, no update checks.
- **Safe for private repositories.** Authentication is handled entirely by your local git config (SSH keys, credential helpers). The tool never sees or stores credentials.
- **Safe for local skill folders.** `skills add ./path` is a pure local copy — nothing leaves your machine.

## Requirements

- Rust 1.70+
- git (on PATH, for remote sources)
