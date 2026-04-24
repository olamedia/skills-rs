use colored::Colorize;

use crate::cmd_add;
use crate::lock::{GlobalLock, LocalLock};

pub struct UpdateArgs {
    pub skills: Vec<String>,
    pub global: bool,
    pub project: bool,
    #[allow(dead_code)]
    pub yes: bool,
}

pub fn run_update(args: UpdateArgs) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let cwd = std::env::current_dir().map_err(|e| format!("{e}"))?;

    let update_global = args.global || (!args.global && !args.project);
    let update_project = args.project || (!args.global && !args.project);

    let mut updated = 0;

    if update_global {
        let lock = GlobalLock::load(&home);
        let entries: Vec<_> = if args.skills.is_empty() {
            lock.skills.values().collect()
        } else {
            lock.skills
                .values()
                .filter(|e| args.skills.contains(&e.skill_name))
                .collect()
        };

        for entry in &entries {
            eprintln!("{} {} (global)", "Updating".cyan(), entry.skill_name);
            let result = cmd_add::run_add(cmd_add::AddArgs {
                source: entry.source_url.clone(),
                global: true,
                yes: true,
                agent_names: entry.agents.clone(),
                skill_names: vec![entry.skill_name.clone()],
                list_only: false,
                copy: false,
                all: false,
            });
            match result {
                Ok(()) => updated += 1,
                Err(e) => eprintln!("  {} {}: {e}", "✗".red(), entry.skill_name),
            }
        }
    }

    if update_project {
        let lock = LocalLock::load(&cwd);
        let entries: Vec<_> = if args.skills.is_empty() {
            lock.skills.values().collect()
        } else {
            lock.skills
                .values()
                .filter(|e| args.skills.contains(&e.skill_name))
                .collect()
        };

        for entry in &entries {
            eprintln!("{} {} (project)", "Updating".cyan(), entry.skill_name);
            let result = cmd_add::run_add(cmd_add::AddArgs {
                source: entry.source_url.clone(),
                global: false,
                yes: true,
                agent_names: entry.agents.clone(),
                skill_names: vec![entry.skill_name.clone()],
                list_only: false,
                copy: false,
                all: false,
            });
            match result {
                Ok(()) => updated += 1,
                Err(e) => eprintln!("  {} {}: {e}", "✗".red(), entry.skill_name),
            }
        }
    }

    if updated == 0 {
        eprintln!("No skills to update.");
    } else {
        eprintln!(
            "\n{} Updated {updated} skill(s).",
            "Done!".green().bold()
        );
    }

    Ok(())
}
