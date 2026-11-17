use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, fs, process::exit};
mod common;
use common::ALL_COMMANDS;
mod functions;
mod utils;
use serde::Deserialize;

#[derive(Deserialize)]
struct Project {
    ignore: Vec<String>,
    remote: String,
    branch: String,
}

#[derive(Deserialize)]
struct Config {
    project: Project,
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

    let conn = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename("D:/kn/hash_db.sqlite")
            .create_if_missing(true),
    )
    .await
    .unwrap();
    let toml = fs::read_to_string("rec.toml").unwrap();
    let project: Config = toml::from_str(&toml).unwrap();
    let project = project.project;

    if args[0] == "s" || args[0] == "save" {
        if args.len() == 1 {
            println!("Too few arguments!")
        } else {
            functions::save(&conn, &args[1], &project).await;
        }
    }
}
