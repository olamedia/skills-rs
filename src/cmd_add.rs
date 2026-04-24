use std::path::Path;

use colored::Colorize;

use crate::agents::{detect_installed_agents, find_agent, AGENTS};
use crate::discover::{discover_skills, filter_skills_by_name};
use crate::git::{cleanup_clone_dir, clone_repo};
use crate::installer::{install_skill, InstallMode};
use crate::lock::{
    hash_skill_dir, now_iso, GlobalLock, GlobalLockEntry, LocalLock, LocalLockEntry,
};
use crate::prompt::{
    confirm_install, is_interactive, print_skill_list, select_agents, select_skills,
};
use crate::source::{parse_source, SourceType};

pub struct AddArgs {
    pub source: String,
    pub global: bool,
    pub yes: bool,
    pub agent_names: Vec<String>,
    pub skill_names: Vec<String>,
    pub list_only: bool,
    pub copy: bool,
    pub all: bool,
}

pub fn run_add(args: AddArgs) -> Result<(), String> {
    let parsed = parse_source(&args.source);

    let skills_root = match parsed.source_type {
        SourceType::Local => {
            let path = parsed.local_path.as_ref().ok_or("Invalid local path")?;
            if !path.exists() {
                return Err(format!("Path does not exist: {}", path.display()));
            }
            path.clone()
        }
        _ => {
            eprintln!("{} {}", "Cloning".cyan(), parsed.url);
            clone_repo(&parsed.url, parsed.git_ref.as_deref())
                .map_err(|e| format!("{e}"))?
        }
    };

    let is_remote = parsed.source_type != SourceType::Local;

    let result = run_add_inner(&args, &parsed, &skills_root);

    if is_remote {
        cleanup_clone_dir(&skills_root);
    }

    result
}

fn run_add_inner(
    args: &AddArgs,
    parsed: &crate::source::ParsedSource,
    skills_root: &Path,
) -> Result<(), String> {
    let all_skills = discover_skills(skills_root, parsed.subpath.as_deref());

    if all_skills.is_empty() {
        return Err(format!(
            "No skills found in {}{}",
            parsed.url,
            parsed
                .subpath
                .as_ref()
                .map(|s| format!("/{s}"))
                .unwrap_or_default()
        ));
    }

    if args.list_only {
        print_skill_list(&all_skills);
        return Ok(());
    }

    // Filter from CLI flag, source fragment, or prompt
    let skill_filter: Vec<String> = if !args.skill_names.is_empty() {
        args.skill_names.clone()
    } else if let Some(ref f) = parsed.skill_filter {
        vec![f.clone()]
    } else {
        vec![]
    };

    let selected_skills = if args.all {
        all_skills.iter().collect::<Vec<_>>()
    } else if !skill_filter.is_empty() {
        let filtered = filter_skills_by_name(&all_skills, &skill_filter);
        if filtered.is_empty() {
            return Err(format!(
                "No matching skills for filter: {}",
                skill_filter.join(", ")
            ));
        }
        filtered
    } else if all_skills.len() == 1 {
        vec![&all_skills[0]]
    } else if is_interactive() && !args.yes {
        let indices = select_skills(&all_skills).map_err(|e| format!("{e}"))?;
        if indices.is_empty() {
            eprintln!("No skills selected.");
            return Ok(());
        }
        indices.iter().map(|&i| &all_skills[i]).collect()
    } else {
        all_skills
            .iter()
            .filter(|s| !s.is_internal)
            .collect::<Vec<_>>()
    };

    if selected_skills.is_empty() {
        return Err("No skills selected.".into());
    }

    // Resolve agents
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let cwd = std::env::current_dir().map_err(|e| format!("{e}"))?;

    let agents: Vec<_> = if !args.agent_names.is_empty() {
        args.agent_names
            .iter()
            .filter_map(|n| find_agent(n))
            .collect()
    } else if args.all || args.yes || !is_interactive() {
        if args.global {
            detect_installed_agents(&home)
        } else {
            AGENTS.iter().collect()
        }
    } else {
        let detected = if args.global {
            detect_installed_agents(&home)
        } else {
            AGENTS.iter().collect()
        };

        if detected.is_empty() {
            return Err("No agents detected. Specify agents with --agent.".into());
        }

        let indices = select_agents(&detected).map_err(|e| format!("{e}"))?;
        if indices.is_empty() {
            eprintln!("No agents selected.");
            return Ok(());
        }
        indices.iter().map(|&i| detected[i]).collect()
    };

    if agents.is_empty() {
        return Err("No agents selected.".into());
    }

    // Confirmation
    let auto_yes = args.yes || args.all;
    if !auto_yes && is_interactive() {
        if !confirm_install(selected_skills.len(), agents.len()).map_err(|e| format!("{e}"))? {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    let mode = if args.copy {
        InstallMode::Copy
    } else {
        InstallMode::Symlink
    };

    // Install + update lock
    let mut installed_count = 0;

    for skill in &selected_skills {
        let results = install_skill(skill, &agents, mode, args.global, &home, &cwd)
            .map_err(|e| format!("{e}"))?;

        let hash = hash_skill_dir(&skill.path).ok();
        let agent_names: Vec<String> = results.iter().map(|r| r.agent_name.clone()).collect();

        if args.global {
            let mut lock = GlobalLock::load(&home);
            lock.upsert(
                &skill.name,
                GlobalLockEntry {
                    source_url: parsed.url.clone(),
                    skill_name: skill.name.clone(),
                    installed_at: now_iso(),
                    skill_folder_hash: hash.clone(),
                    agents: agent_names.clone(),
                },
            );
            lock.save(&home).map_err(|e| format!("{e}"))?;
        } else {
            let mut lock = LocalLock::load(&cwd);
            lock.upsert(
                &skill.name,
                LocalLockEntry {
                    source_url: parsed.url.clone(),
                    skill_name: skill.name.clone(),
                    installed_at: now_iso(),
                    computed_hash: hash.clone(),
                    agents: agent_names.clone(),
                },
            );
            lock.save(&cwd).map_err(|e| format!("{e}"))?;
        }

        for result in &results {
            eprintln!(
                "  {} {} → {} ({})",
                "✓".green(),
                result.skill_name.cyan(),
                result.agent_name.bold(),
                result.dest.display()
            );
        }
        installed_count += 1;
    }

    eprintln!(
        "\n{} Installed {} skill(s) to {} agent(s).",
        "Done!".green().bold(),
        installed_count,
        agents.len()
    );

    Ok(())
}
