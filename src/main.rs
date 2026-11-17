use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, fs, process::exit};
mod common;
use common::ALL_COMMANDS;
mod commands;
mod utils;
use serde::Deserialize;

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

    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename("D:/kn/hash_db.sqlite")
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

    sqlx::query("CREATE TABLE IF NOT EXISTS hashes (id TEXT PRIMARY KEY NOT NULL, timestamp DATETIME NOT NULL, project TEXT NOT NULL, branch TEXT NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL, comment TEXT NOT NULL);")
        .execute(&db)
        .await.unwrap();

    if args[0] == "s" || args[0] == "save" {
        if args.len() != 2 {
            eprintln!("Wrong number of arguments! (1) required: rec save <message>");
        } else {
            println!("Saving files...");
            commands::save(&db, &args[1], &project).await.unwrap();
        }
    } else {
        eprintln!("Unknown rec command: {}", args[0])
    }
}
