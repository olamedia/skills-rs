use std::io;

use colored::Colorize;
use dialoguer::{Confirm, MultiSelect};

use crate::agents::AgentConfig;
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

    let defaults: Vec<bool> = skills.iter().map(|s| !s.is_internal).collect();

    let selected = MultiSelect::new()
        .with_prompt("Select skills to install")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(selected)
}

pub fn select_agents(agents: &[&AgentConfig]) -> io::Result<Vec<usize>> {
    if agents.is_empty() {
        return Ok(vec![]);
    }

    let items: Vec<String> = agents.iter().map(|a| a.display_name.to_string()).collect();
    let defaults: Vec<bool> = vec![true; agents.len()];

    let selected = MultiSelect::new()
        .with_prompt("Select target agents")
        .items(&items)
        .defaults(&defaults)
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
