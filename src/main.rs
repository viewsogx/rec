use sqlx::{AssertSqlSafe, SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, fs, path::Path, process::exit};
mod common;
use common::ALL_COMMANDS;
mod commands;
mod utils;
use serde::Deserialize;

use crate::utils::{generate_id, get_timestamp};

#[derive(Deserialize)]
struct Project {
    ignore: Vec<String>,
    name: String,
    remote: String,
    branch: String,
    id: Option<String>,
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

    let Ok(toml) = fs::read_to_string("rec.toml") else {
        eprintln!("rec.toml file couldn't be found.");
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
        "CREATE TABLE IF NOT EXISTS 'hashes_{}' (id INTEGER PRIMARY KEY AUTOINCREMENT, save_id TEXT NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL);",
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
    if args[0] == "s" || args[0] == "save" {
        if args.len() != 2 {
            eprintln!("Wrong number of arguments! (1) required: rec save <message>");
        } else {
            println!("Saving files...");
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
    } else {
        eprintln!("Unknown rec command: {}", args[0])
    }
}
