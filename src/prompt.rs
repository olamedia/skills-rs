use std::io;

use colored::Colorize;
use dialoguer::{Confirm, MultiSelect, Select};

use crate::agents::AgentConfig;
use crate::installer::InstallMode;
use crate::skill::Skill;

pub fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}

pub fn select_skills(skills: &[Skill]) -> io::Result<Vec<usize>> {
    if skills.is_empty() {
        return Ok(vec![]);
    }

    let items: Vec<String> = skills
        .iter()
        .map(|s| {
            if s.is_internal {
                format!("{} {} (internal)", s.name, s.description.dimmed())
            } else {
                format!("{} {}", s.name, s.description.dimmed())
            }
        })
        .collect();

    let selected = MultiSelect::new()
        .with_prompt("Select skills to install")
        .items(&items)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(selected)
}

pub fn select_agents(agents: &[&AgentConfig], saved_agents: &[String]) -> io::Result<Vec<usize>> {
    if agents.is_empty() {
        return Ok(vec![]);
    }

    let items: Vec<String> = agents.iter().map(|a| a.display_name.to_string()).collect();
    let defaults: Vec<bool> = agents
        .iter()
        .map(|a| saved_agents.iter().any(|s| s == a.name))
        .collect();

    let has_saved = defaults.iter().any(|&d| d);
    let ms = MultiSelect::new()
        .with_prompt("Select target agents")
        .items(&items);
    let selected = if has_saved {
        ms.defaults(&defaults)
    } else {
        ms
    }
    .interact()
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(selected)
}

pub fn confirm_install(count: usize, agent_count: usize) -> io::Result<bool> {
    Confirm::new()
        .with_prompt(format!(
            "Install {count} skill(s) to {agent_count} agent(s)?"
        ))
        .default(true)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn prompt_install_to_home(saved: Option<bool>) -> io::Result<bool> {
    let c = Confirm::new()
        .with_prompt("Install skills to home (~/.agents/skills/)?");
    let c = match saved {
        Some(v) => c.default(v),
        None => c,
    };
    c.interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn prompt_install_mode(saved: Option<&str>) -> io::Result<InstallMode> {
    let items = &["Symlink (recommended)", "Copy"];
    let default_idx = match saved {
        Some("copy") => 1,
        _ => 0,
    };

    let selection = Select::new()
        .with_prompt("Installation method for project")
        .items(items)
        .default(default_idx)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(if selection == 0 {
        InstallMode::Symlink
    } else {
        InstallMode::Copy
    })
}

pub fn print_skill_list(skills: &[Skill]) {
    println!("\n{}", "Available skills:".bold());
    for skill in skills {
        let internal_tag = if skill.is_internal { " (internal)" } else { "" };
        println!(
            "  {} {}{}",
            skill.name.cyan(),
            skill.description.dimmed(),
            internal_tag.yellow()
        );
    }
    println!();
}
