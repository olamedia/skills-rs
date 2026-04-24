mod agents;
mod cmd_add;
mod cmd_init;
mod cmd_list;
mod cmd_remove;
mod cmd_update;
mod discover;
mod git;
mod installer;
mod lock;
mod prompt;
mod sanitize;
mod skill;
mod source;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skills", version, about = "Install agent skills from any git source")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add skills from a repository or local path
    #[command(alias = "a", alias = "install", alias = "i")]
    Add {
        /// Source: owner/repo, URL, or local path
        source: String,

        #[arg(short, long, help = "Install globally (user-level)")]
        global: bool,

        #[arg(short, long, help = "Skip confirmation prompts")]
        yes: bool,

        #[arg(short, long, num_args = 1.., help = "Target agents")]
        agent: Vec<String>,

        #[arg(short, long, num_args = 1.., help = "Skills to install")]
        skill: Vec<String>,

        #[arg(short, long, help = "List available skills without installing")]
        list: bool,

        #[arg(long, help = "Copy instead of symlink")]
        copy: bool,

        #[arg(long, help = "Install all skills to all agents (implies -y)")]
        all: bool,
    },

    /// List installed skills
    #[command(alias = "ls")]
    List {
        #[arg(short, long, help = "List global skills")]
        global: bool,

        #[arg(short, long, num_args = 1.., help = "Filter by agent")]
        agent: Vec<String>,

        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    /// Remove installed skills
    #[command(alias = "rm", alias = "r")]
    Remove {
        /// Skill names to remove (interactive if omitted)
        skills: Vec<String>,

        #[arg(short, long, help = "Remove from global scope")]
        global: bool,

        #[arg(short, long, num_args = 1.., help = "Remove from specific agents")]
        agent: Vec<String>,

        #[arg(short, long, help = "Skip confirmation")]
        yes: bool,

        #[arg(long, help = "Remove all skills from all agents")]
        all: bool,
    },

    /// Update installed skills to latest versions
    #[command(alias = "upgrade")]
    Update {
        /// Skill names to update (all if omitted)
        skills: Vec<String>,

        #[arg(short, long, help = "Update global skills only")]
        global: bool,

        #[arg(short, long, help = "Update project skills only")]
        project: bool,

        #[arg(short, long, help = "Skip prompts")]
        yes: bool,
    },

    /// Create a new SKILL.md template
    Init {
        /// Skill name (creates subdirectory)
        name: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add {
            source,
            global,
            yes,
            agent,
            skill,
            list,
            copy,
            all,
        }) => {
            let result = cmd_add::run_add(cmd_add::AddArgs {
                source,
                global,
                yes,
                agent_names: agent,
                skill_names: skill,
                list_only: list,
                copy,
                all,
            });
            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::List {
            global,
            agent,
            json,
        }) => {
            let result = cmd_list::run_list(cmd_list::ListArgs {
                global,
                agent_names: agent,
                json,
            });
            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::Remove {
            skills,
            global,
            agent,
            yes,
            all,
        }) => {
            let result = cmd_remove::run_remove(cmd_remove::RemoveArgs {
                skills,
                global,
                agent_names: agent,
                yes,
                all,
            });
            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::Update {
            skills,
            global,
            project,
            yes,
        }) => {
            let result = cmd_update::run_update(cmd_update::UpdateArgs {
                skills,
                global,
                project,
                yes,
            });
            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::Init { name }) => {
            let result = cmd_init::run_init(cmd_init::InitArgs { name });
            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        None => {
            eprintln!("skills - Install agent skills from any git source\n");
            eprintln!("Usage: skills <command> [options]\n");
            eprintln!("Commands:");
            eprintln!("  add <source>     Add skills from a repository");
            eprintln!("  list             List installed skills");
            eprintln!("  remove [skills]  Remove installed skills");
            eprintln!("  update [skills]  Update skills to latest");
            eprintln!("  init [name]      Create a new skill template");
            eprintln!("\nRun `skills --help` for details.");
        }
    }
}
