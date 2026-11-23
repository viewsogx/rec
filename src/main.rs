mod commands;
mod common;
mod utils;

use crate::common::Config;
use crate::utils::generate_id;
use colored::Colorize;
use common::ALL_COMMANDS;
use sqlx::{AssertSqlSafe, SqlitePool, sqlite::SqliteConnectOptions};
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

#[tokio::main]
async fn main() {
    colored::control::set_override(true);

    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();

    let mut args: Vec<String> = env::args().collect();
    args.remove(0);

    if args.len() == 0 {
        print_all_commands();
        exit(0)
    }

    let command = args[0].trim();

    let data_dir = dirs::data_dir().expect("no data dir").join(".rec");

    fs::create_dir_all(data_dir.join("data")).unwrap();
    let hashes_location = data_dir.join("data").join("data.sqlite");

    #[cfg(windows)]
    hf::hide(&data_dir).unwrap();

    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(hashes_location)
            .create_if_missing(true),
    )
    .await
    .unwrap();

    if (command == "i" || command == "init") && args.len() > 1 {
        eprintln!(
            "Wrong number of arguments! {} required:\n{} init",
            "(0)".red(),
            "rec".yellow()
        );
        return;
    }

    let Ok(toml) = fs::read_to_string("rec.toml") else {
        if command == "i" || command == "init" {
            if let Err(err) = commands::init() {
                eprintln!("Failed to initialize project:\n{}", err.to_string());
            };
            return;
        }
        eprintln!("{} file couldn't be found.", "rec.toml".red());
        println!(
            "Initialize {} in current directory? (y/N)",
            "rec project".yellow()
        );
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).unwrap();

        if answer.trim().to_lowercase() == "y" {
            if let Err(err) = commands::init() {
                eprintln!("Failed to initialize project:\n{}", err.to_string());
                return;
            };
            println!(
                "{}",
                "Successfully initialized rec project (rec.toml created)".bright_green()
            );
            return;
        }
        return;
    };

    let Ok(project): Result<Config, _> = toml::from_str(&toml) else {
        eprintln!("Parsing error: Invalid rec.toml file");
        return;
    };
    let mut project = project.config;

    // GENERATE PROJECT ID
    if project.id.is_none() {
        let proj_id = generate_id(8);
        fs::write("rec.toml", format!("{toml}\nid = \"{proj_id}\"")).unwrap();
        project.id = Some(proj_id);
    }
    let project = project;

    let project_id = project.id.as_ref().unwrap();
    let create_hash_table = format!(
        r#"CREATE TABLE IF NOT EXISTS "hashes_{project_id}" (id TEXT PRIMARY KEY, save_id TEXT NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL, size TEXT NOT NULL, location TEXT, cdata BLOB);
        CREATE INDEX "idx_hashes_{project_id}"
        ON "hashes_{project_id}"(hash);
        "#
    );
    let create_save_table = format!(
        r#"CREATE TABLE IF NOT EXISTS "saves_{project_id}" (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, branch TEXT NOT NULL, comment TEXT NOT NULL);"#
    );
    sqlx::query(AssertSqlSafe(create_hash_table))
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(AssertSqlSafe(create_save_table))
        .execute(&db)
        .await
        .unwrap();

    // handle commands
    match command {
        "s" | "save" => {
            if args.len() != 2 {
                eprintln!(
                    "Wrong number of arguments! {} required:\n{} save {}",
                    "(1)".red(),
                    "rec".yellow(),
                    "<message>".cyan()
                );
            } else {
                println!("{}", "Saving files...".bright_green());
                if let Err(error) =
                    commands::save(&db, Path::new("."), &args[1], &project, &generate_id(8)).await
                {
                    eprintln!("{error}")
                }
            }
        }
        "i" | "init" => {
            println!(
                "{} {}",
                "rec project".yellow(),
                "already initialized".bright_blue()
            )
        }
        "ld" | "load" => {
            if args.len() != 2 {
                eprintln!(
                    "Wrong number of arguments! {} required:\n{} load {}",
                    "(1)".red(),
                    "rec".yellow(),
                    "<save_id>".cyan()
                );
            } else {
                let save_id = &args[1];
                if let Err(error) = commands::load(&db, save_id, &project).await {
                    eprintln!("{error}");
                    return;
                }
                println!("Successfully restored save {}.", save_id.bright_green());
            }
        }
        "l" | "log" => {
            if args.len() != 2 {
                eprintln!(
                    "Wrong number of arguments! {} required:\n{} log {}",
                    "(1)".red(),
                    "rec".yellow(),
                    "<limit>".cyan()
                );
            } else {
                if let Ok(limit) = args[1].parse() {
                    if limit <= 0 {
                        eprintln!("Limit must be a positive number.")
                    } else {
                        match commands::log(&db, &project, limit).await {
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
                } else {
                    eprintln!("Limit must be a number.")
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command.yellow());
            let mut suggested = String::from("");
            for cmd in ALL_COMMANDS {
                if cmd.name.contains(&command) {
                    suggested.push_str("\n");
                    suggested.push_str(&cmd.name);
                }
            }
            if !suggested.is_empty() {
                println!("Suggested existing commands: {}", suggested.cyan())
            }
        }
    }
}
