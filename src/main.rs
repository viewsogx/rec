use std::{env, fs};
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

fn main() {
    let mut args: Vec<String> = env::args().collect();

    args.remove(0);
    if args.len() == 0 {
        print_all_commands();
    }
}
