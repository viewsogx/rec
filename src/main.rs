mod commands;
mod common;
mod utils;

use crate::common::{Config, Project};
use crate::utils::generate_id;
use colored::Colorize;
use common::ALL_COMMANDS;
use rusqlite::Connection;
use std::{env, fs, io, path::Path, process::exit};

fn print_all_commands() {
    println!(
        "{}",
        "\n\n----------- rec - HELP -----------\n".on_bright_black()
    );
    for command in ALL_COMMANDS {
        println!(
            "\t{}\t{}\t- {}",
            command.name.yellow(),
            command.short.bright_yellow(),
            command.description
        )
    }
    println!("")
}

fn print_unknown_command(command: &str) {
    eprintln!("Unknown command: {}", command.yellow());
    let mut suggested = String::from("");
    for cmd in ALL_COMMANDS {
        if cmd.name.contains(command) {
            suggested.push_str("\n");
            suggested.push_str(&cmd.name);
        }
    }
    if !suggested.is_empty() {
        println!("Suggested existing commands: {}", suggested.cyan())
    }
}

fn init_command(args: &[String]) {
    if args.len() > 1 {
        eprintln!(
            "Wrong number of arguments! {} required:\n{} init",
            "(0)".red(),
            "rec".yellow()
        );
        return;
    }

    if Path::new("rec.toml").exists() {
        println!(
            "{} {}",
            "rec project".yellow(),
            "already initialized".bright_blue()
        );
        return;
    }

    if let Err(err) = commands::init() {
        eprintln!("Failed to initialize project:\n{}", err.to_string());
        return;
    }
    println!("{}", "Successfully initialized project.".bright_green());
}

fn load_project() -> Option<Project> {
    let Ok(toml) = fs::read_to_string("rec.toml") else {
        eprintln!("{} file couldn't be found.", "rec.toml".red());
        println!(
            "Initialize {} in current directory? (y/N)",
            "rec project".yellow()
        );
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).unwrap();

        if answer.trim().to_lowercase() == "y" {
            match commands::init() {
                Ok(()) => println!(
                    "{}",
                    "Successfully initialized rec project (rec.toml created)".bright_green()
                ),
                Err(err) => eprintln!("Failed to initialize project:\n{}", err.to_string()),
            }
        }
        return None;
    };

    let Ok(config): Result<Config, _> = toml::from_str(&toml) else {
        eprintln!("Parsing error: Invalid rec.toml file");
        return None;
    };
    let mut project = config.config;

    // GENERATE PROJECT ID
    if project.id.is_none() {
        let proj_id = generate_id(8);
        if let Err(err) = fs::write("rec.toml", format!("{toml}\nid = \"{proj_id}\"")) {
            eprintln!("Failed to update rec.toml:\n{}", err.to_string());
            return None;
        }
        project.id = Some(proj_id);
    }

    Some(project)
}

fn open_db() -> Option<Connection> {
    let data_dir = dirs::data_dir().expect("no data dir").join(".rec");

    if let Err(err) = fs::create_dir_all(data_dir.join("data")) {
        eprintln!("Couldn't create data directory:\n{}", err.to_string());
        return None;
    }

    #[cfg(windows)]
    hf::hide(&data_dir).unwrap();

    match Connection::open(data_dir.join("data").join("data.sqlite")) {
        Ok(db) => Some(db),
        Err(_) => {
            eprintln!("Couldn't connect to local database.");
            None
        }
    }
}

fn create_project_tables(db: &Connection, project: &Project) -> bool {
    let project_id = project.id.as_ref().unwrap();
    let create_hash_table = format!(
        r#"CREATE TABLE IF NOT EXISTS "hashes_{project_id}" (id TEXT PRIMARY KEY, save_id TEXT NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL, size TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS "idx_hashes_{project_id}"
        ON "hashes_{project_id}"(hash);
        "#
    );
    let create_save_table = format!(
        r#"CREATE TABLE IF NOT EXISTS "saves_{project_id}" (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, branch TEXT NOT NULL, comment TEXT NOT NULL);"#
    );

    if let Err(err) = db
        .execute_batch(&create_hash_table)
        .and_then(|_| db.execute(&create_save_table, []))
    {
        eprintln!("Couldn't prepare local database:\n{}", err.to_string());
        return false;
    }

    true
}

fn open_project() -> Option<(Connection, Project)> {
    let project = load_project()?;
    let db = open_db()?;

    if !create_project_tables(&db, &project) {
        return None;
    }

    Some((db, project))
}

fn save_command(args: &[String]) {
    if args.len() != 2 {
        eprintln!(
            "Wrong number of arguments! {} required:\n{} save {}",
            "(1)".red(),
            "rec".yellow(),
            "<message>".cyan()
        );
        return;
    }

    let Some((db, project)) = open_project() else {
        return;
    };

    println!("{}", "Saving files...".bright_green());
    if let Err(error) = commands::save(&db, &args[1], &project) {
        eprintln!("{error}")
    }
}

fn load_command(args: &[String]) {
    if args.len() != 2 {
        eprintln!(
            "Wrong number of arguments! {} required:\n{} load {}",
            "(1)".red(),
            "rec".yellow(),
            "<save_id>".cyan()
        );
        return;
    }

    let save_id = &args[1];
    let Some((db, project)) = open_project() else {
        return;
    };

    if let Err(error) = commands::load(&db, save_id, &project) {
        eprintln!("{error}");
        return;
    }
    println!("Successfully restored save {}.", save_id.bright_green());
}

fn log_command(args: &[String]) {
    if args.len() != 2 {
        eprintln!(
            "Wrong number of arguments! {} required:\n{} log {}",
            "(1)".red(),
            "rec".yellow(),
            "<limit>".cyan()
        );
        return;
    }

    let Ok(limit): Result<u32, _> = args[1].parse() else {
        eprintln!("Limit must be a number.");
        return;
    };
    if limit == 0 {
        eprintln!("Limit must be a positive number.");
        return;
    }

    let Some((db, project)) = open_project() else {
        return;
    };

    match commands::log(&db, &project, limit) {
        Err(err) => {
            eprintln!("{err}")
        }
        Ok(saves) => {
            if saves.len() == 0 {
                println!("No saves found for project: {}", project.name)
            } else {
                println!(
                    "\nSAVE LOGS - {} - showing last {limit} saves\n",
                    project.name.yellow()
                );
                println!("{}", "id\t\ttimestamp\tbranch\tcomment".red());
                for save in saves {
                    if save.comment.len() > 20 {
                        println!(
                            "{}\t{}\t{}\t{}...",
                            save.id,
                            save.timestamp,
                            save.branch,
                            &save.comment[..20]
                        )
                    } else {
                        println!(
                            "{}\t{}\t{}\t{}",
                            save.id, save.timestamp, save.branch, save.comment
                        )
                    }
                }
            }
        }
    }
}

fn main() {
    colored::control::set_override(true);

    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();

    let mut args: Vec<String> = env::args().collect();
    args.remove(0);

    let Some(command) = args.first().map(|command| command.trim()) else {
        print_all_commands();
        exit(0)
    };

    match command {
        "i" | "init" => init_command(&args),
        "s" | "save" => save_command(&args),
        "ld" | "load" => load_command(&args),
        "l" | "log" => log_command(&args),
        _ => print_unknown_command(command),
    }
}
