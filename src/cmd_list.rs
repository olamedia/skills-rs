use colored::Colorize;

use crate::lock::{GlobalLock, LocalLock};

pub struct ListArgs {
    pub global: bool,
    pub agent_names: Vec<String>,
    pub json: bool,
}

pub fn run_list(args: ListArgs) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let cwd = std::env::current_dir().map_err(|e| format!("{e}"))?;

    if args.global {
        let lock = GlobalLock::load(&home);
        if args.json {
            let json = serde_json::to_string_pretty(&lock).map_err(|e| format!("{e}"))?;
            println!("{json}");
            return Ok(());
        }
        print_global_lock(&lock, &args.agent_names);
    } else {
        let lock = LocalLock::load(&cwd);
        if args.json {
            let json = serde_json::to_string_pretty(&lock).map_err(|e| format!("{e}"))?;
            println!("{json}");
            return Ok(());
        }
        print_local_lock(&lock, &args.agent_names);
    }

    Ok(())
}

fn print_global_lock(lock: &GlobalLock, agent_filter: &[String]) {
    let filtered: Vec<_> = lock
        .skills
        .values()
        .filter(|e| {
            agent_filter.is_empty()
                || e.agents.iter().any(|a| agent_filter.contains(a))
        })
        .collect();

    if filtered.is_empty() {
        eprintln!("No global skills installed.");
        return;
    }

    eprintln!("{}", "Global skills:".bold());
    for entry in &filtered {
        eprintln!(
            "  {} {} [{}]",
            entry.skill_name.cyan(),
            format!("from {}", entry.source_url).dimmed(),
            entry.agents.join(", ")
        );
    }
}

fn print_local_lock(lock: &LocalLock, agent_filter: &[String]) {
    let filtered: Vec<_> = lock
        .skills
        .values()
        .filter(|e| {
            agent_filter.is_empty()
                || e.agents.iter().any(|a| agent_filter.contains(a))
        })
        .collect();

    if filtered.is_empty() {
        eprintln!("No project skills installed.");
        return;
    }

    eprintln!("{}", "Project skills:".bold());
    for entry in &filtered {
        eprintln!(
            "  {} {} [{}]",
            entry.skill_name.cyan(),
            format!("from {}", entry.source_url).dimmed(),
            entry.agents.join(", ")
        );
    }
}
