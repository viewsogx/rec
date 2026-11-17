use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, process::exit};
mod common;
use common::ALL_COMMANDS;
mod functions;
mod utils;

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
            .filename("hash_db.db")
            .create_if_missing(true),
    )
    .await
    .unwrap();
    if args[0] == "s" || args[0] == "save" {
        functions::save(&conn, &args[1]).await;
    }
}
