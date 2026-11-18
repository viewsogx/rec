use sqlx::{AssertSqlSafe, SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, fs, path::Path, process::exit};
mod common;
use common::ALL_COMMANDS;
mod commands;
mod utils;
use serde::Deserialize;

use crate::utils::generate_id;

#[derive(Deserialize)]
struct Project {
    ignore: Vec<String>,
    name: String,
    remote: String,
    branch: String,
}

#[derive(Deserialize)]
struct Config {
    config: Project,
}

fn print_all_commands() {
    println!("\n----------- rec - HELP -----------\n");
    for command in ALL_COMMANDS {
        println!(
            "\t{}\t{}\t- {}",
            command.name, command.short, command.description
        )
    }
    println!("")
}

#[tokio::main]
async fn main() {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);

    if args.len() == 0 {
        print_all_commands();
        exit(0)
    }

    let hashes_location = if env::consts::OS == "windows" {
        "C:/Program Files/rec/data/data.sqlite"
    } else {
        "/var/lib/rec/data/data.sqlite"
    };

    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(hashes_location)
            .create_if_missing(true),
    )
    .await
    .unwrap();
    let search_toml = fs::read_to_string("rec.toml");
    if search_toml.is_err() {
        eprintln!("rec.toml file couldn't be found.");
        return;
    };
    let toml = search_toml.unwrap();
    let Ok(project): Result<Config, _> = toml::from_str(&toml) else {
        eprintln!("Parsing error: Invalid rec.toml file");
        return;
    };
    let project = project.config;
    let create_hash_table = format!(
        "CREATE TABLE IF NOT EXISTS 'hashes_{}' (id INTEGER PRIMARY KEY  AUTOINCREMENT, save_id TEXT NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL);",
        &project.name
    );
    let create_save_table = format!(
        "CREATE TABLE IF NOT EXISTS 'saves_{}' (id TEXT PRIMARY KEY NOT NULL, timestamp DATETIME NOT NULL, branch TEXT NOT NULL, comment TEXT NOT NULL);",
        &project.name
    );
    sqlx::query(AssertSqlSafe(create_hash_table))
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(AssertSqlSafe(create_save_table))
        .execute(&db)
        .await
        .unwrap();
    if args[0] == "s" || args[0] == "save" {
        if args.len() != 2 {
            eprintln!("Wrong number of arguments! (1) required: rec save <message>");
        } else {
            println!("Saving files...");
            commands::save(Path::new("."), &db, &args[1], &project, &generate_id(10))
                .await
                .unwrap();
        }
    } else {
        eprintln!("Unknown rec command: {}", args[0])
    }
}
