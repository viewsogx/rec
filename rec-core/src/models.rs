use rusqlite_from_row::FromRow;
use serde::{Deserialize, Serialize};

pub struct Command {
    pub name: &'static str,
    pub short: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Project {
    pub ignore: Vec<String>,
    pub name: String,
    pub remote: String,
    pub branch: String,
    pub id: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub config: Project,
}

#[derive(FromRow, Debug)]
pub struct File {
    pub id: String,
    pub save_id: String,
    pub file_name: String,
    pub hash: String,
    pub size: String,
}

#[derive(Debug)]
pub struct ShortFile {
    pub file_name: String,
    pub size: u64,
}

#[derive(FromRow, Debug)]
pub struct Save {
    pub id: String,
    pub timestamp: String,
    pub branch: String,
    pub comment: String,
}
