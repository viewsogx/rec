use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{env, fs, process::exit};
mod common;
use common::ALL_COMMANDS;

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

    let file = fs::read("src/main.rs").unwrap();
    let hash = blake3::hash(&file);
    println!("{hash:?}");
    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename("hash_db.db")
            .create_if_missing(true),
    )
    .await
    .unwrap();

    println!("{:?}", env::current_dir());
}
