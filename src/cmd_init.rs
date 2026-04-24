use std::fs;
use std::path::Path;

use colored::Colorize;

pub struct InitArgs {
    pub name: Option<String>,
}

const TEMPLATE: &str = r#"---
name: {NAME}
description: Brief description of what this skill does
---

# {NAME}

## When to Use

Describe when this skill should be activated.

## Instructions

1. Step one
2. Step two
3. Step three
"#;

pub fn run_init(args: InitArgs) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("{e}"))?;

    let (dir, name) = if let Some(ref n) = args.name {
        (cwd.join(n), n.clone())
    } else {
        let dir_name = cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-skill")
            .to_string();
        (cwd.clone(), dir_name)
    };

    let skill_md = dir.join("SKILL.md");
    if skill_md.exists() {
        return Err(format!("SKILL.md already exists at {}", skill_md.display()));
    }

    create_skill_template(&dir, &name)?;

    eprintln!(
        "{} Created {} at {}",
        "✓".green(),
        "SKILL.md".cyan(),
        skill_md.display()
    );

    Ok(())
}

fn create_skill_template(dir: &Path, name: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("{e}"))?;
    let content = TEMPLATE.replace("{NAME}", name);
    fs::write(dir.join("SKILL.md"), content).map_err(|e| format!("{e}"))?;
    Ok(())
}
