use colored::Colorize;
use dialoguer::MultiSelect;

use crate::agents::{find_agent, AGENTS};
use crate::installer::{remove_canonical, remove_skill};
use crate::lock::{GlobalLock, LocalLock};
use crate::prompt::is_interactive;

pub struct RemoveArgs {
    pub skills: Vec<String>,
    pub global: bool,
    pub agent_names: Vec<String>,
    pub yes: bool,
    pub all: bool,
}

pub fn run_remove(args: RemoveArgs) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let cwd = std::env::current_dir().map_err(|e| format!("{e}"))?;

    let skill_names = if args.all {
        let lock = if args.global {
            let g = GlobalLock::load(&home);
            g.skills.keys().cloned().collect::<Vec<_>>()
        } else {
            let l = LocalLock::load(&cwd);
            l.skills.keys().cloned().collect::<Vec<_>>()
        };
        lock
    } else if !args.skills.is_empty() {
        args.skills.clone()
    } else if is_interactive() {
        let lock_keys: Vec<String> = if args.global {
            GlobalLock::load(&home).skills.keys().cloned().collect()
        } else {
            LocalLock::load(&cwd).skills.keys().cloned().collect()
        };

        if lock_keys.is_empty() {
            eprintln!("No skills installed to remove.");
            return Ok(());
        }

        let selected = MultiSelect::new()
            .with_prompt("Select skills to remove")
            .items(&lock_keys)
            .interact()
            .map_err(|e| format!("{e}"))?;

        selected.iter().map(|&i| lock_keys[i].clone()).collect()
    } else {
        return Err("No skills specified. Use --all or provide names.".into());
    };

    if skill_names.is_empty() {
        eprintln!("No skills selected.");
        return Ok(());
    }

    // Confirmation
    if !args.yes && !args.all && is_interactive() {
        eprintln!(
            "Will remove: {}",
            skill_names.join(", ").cyan()
        );
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Proceed?")
            .default(true)
            .interact()
            .map_err(|e| format!("{e}"))?;
        if !confirm {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    let agents: Vec<_> = if !args.agent_names.is_empty() {
        args.agent_names.iter().filter_map(|n| find_agent(n)).collect()
    } else {
        AGENTS.iter().collect()
    };

    let mut removed = 0;
    for name in &skill_names {
        for agent in &agents {
            let _ = remove_skill(name, agent, args.global, &home, &cwd);
        }
        let _ = remove_canonical(name, args.global, &home, &cwd);

        if args.global {
            let mut lock = GlobalLock::load(&home);
            lock.remove(name);
            lock.save(&home).map_err(|e| format!("{e}"))?;
        } else {
            let mut lock = LocalLock::load(&cwd);
            lock.remove(name);
            lock.save(&cwd).map_err(|e| format!("{e}"))?;
        }

        eprintln!("  {} removed {}", "✓".green(), name.cyan());
        removed += 1;
    }

    eprintln!("\n{} Removed {removed} skill(s).", "Done!".green().bold());
    Ok(())
}
