use colored::Colorize;
use sqlx::{AssertSqlSafe, SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, fs, io, path::Path, process::exit};
mod common;
use common::ALL_COMMANDS;
mod commands;
mod utils;
use serde::{Deserialize, Serialize};

use crate::utils::{generate_id, get_timestamp};

#[derive(Deserialize, Debug, Serialize, Default)]
struct Project {
    ignore: Vec<String>,
    name: String,
    remote: String,
    branch: String,
    id: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Config {
    config: Project,
}

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
    colored::control::set_virtual_terminal(true).unwrap();
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);

    if args.len() == 0 {
        print_all_commands();
        exit(0)
    }

    let hashes_location = if env::consts::OS == "windows" {
        fs::create_dir_all("C:/ProgramData/.rec/data/").unwrap();
        hf::hide("C:/ProgramData/.rec/").unwrap();
        "C:/ProgramData/.rec/data/data.sqlite"
    } else if env::consts::OS == "macos" {
        fs::create_dir_all("/Library/Application Support/rec/data/").unwrap();
        "/Library/Application Support/.rec/data/data.sqlite"
    } else {
        fs::create_dir_all("/var/lib/rec/data/").unwrap();
        "/var/lib/.rec/data/data.sqlite"
    };

    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(hashes_location)
            .create_if_missing(true),
    )
    .await
    .unwrap();

    if (args[0] == "i" || args[0] == "init") && args.len() > 1 {
        eprintln!(
            "Wrong number of arguments! {} required:\n{} init",
            "(0)".red(),
            "rec".yellow()
        );
        return;
    }

    let Ok(toml) = fs::read_to_string("rec.toml") else {
        if args[0] == "i" || args[0] == "init" {
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
            };
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
        fs::write(
            "rec.toml",
            format!("{}\nid = {}", toml, format!(r#""{}""#, proj_id)),
        )
        .unwrap();
        project.id = Some(proj_id);
    }
    let project = project;

    let create_hash_table = format!(
        "CREATE TABLE IF NOT EXISTS 'hashes_{}' (id INTEGER PRIMARY KEY AUTOINCREMENT, save_id TEXT NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL, size TEXT NOT NULL, cdata BLOB NOT NULL);",
        &project.id.as_ref().unwrap()
    );
    let create_save_table = format!(
        "CREATE TABLE IF NOT EXISTS 'saves_{}' (id TEXT PRIMARY KEY NOT NULL, timestamp DATETIME NOT NULL, branch TEXT NOT NULL, comment TEXT NOT NULL);",
        &project.id.as_ref().unwrap()
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
    match args[0].as_str() {
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
                let save_id = generate_id(8);
                let comment = &args[1];
                let insert_query = format!(
                    "INSERT INTO 'saves_{}'
            ( id, timestamp, branch, comment)
            VALUES ( ?, ?, ?, ?)",
                    &project.id.as_ref().unwrap()
                );
                sqlx::query(AssertSqlSafe(insert_query))
                    .bind(&save_id)
                    .bind(get_timestamp())
                    .bind(&project.branch)
                    .bind(&comment)
                    .execute(&db)
                    .await
                    .unwrap();

                commands::save(&db, Path::new("."), comment, &project, &save_id)
                    .await
                    .unwrap();
            }
        }
        "i" | "init" => {
            println!(
                "{} {}",
                "rec project".yellow(),
                "already initialized".bright_blue()
            )
        }
        _ => {
            eprintln!("Unknown command: {}", args[0].yellow());
            ALL_COMMANDS.iter().for_each(|cmd| {
                if cmd.name.contains(&args[0]) {
                    println!("Suggested existing command: {}", cmd.name.cyan())
                }
            });
        }
    }
}
