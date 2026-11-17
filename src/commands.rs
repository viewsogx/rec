use std::path::Path;

use sqlx::{Pool, Sqlite};
use std::fs;

use crate::{
    Project,
    utils::{generate_id, get_timestamp},
};

pub async fn save(db: &Pool<Sqlite>, comment: &str, project: &Project) {
    // TODO: FOR EACH FILE THAT ISNT IN IGNORE DO THIS WHOLE THING
    let mut entries = fs::read_dir(".").unwrap();

    while let Ok(entry) = entries.next().unwrap() {
        let file = fs::read(&entry.path()).unwrap();
        let hash = blake3::hash(&file);

        sqlx::query("INSERT INTO hashes (id, timestamp, project, branch, file_name, hash, comment) VALUES (?,?, ?, ?, ?, ?, ?)")
        .bind(generate_id(8))
        .bind(get_timestamp())
        .bind(&project.name)
        .bind(&project.branch)
        .bind(&entry.path().to_str())
        .bind(&hash.to_string())
        .bind(&comment)
        .execute(db)
        .await
        .unwrap();
    }
}
