use sqlx::{Pool, Sqlite};
use tokio::fs;

use crate::{Project, utils::get_timestamp};

pub async fn save(conn: &Pool<Sqlite>, comment: &str, project: &Project) {
    // TODO: FOR EACH FILE THAT ISNT IN IGNORE DO THIS WHOLE THING
    let file = fs::read(&file_name).await.unwrap();
    let hash = blake3::hash(&file);

    sqlx::query("CREATE TABLE IF NOT EXISTS hashes (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp DATETIME NOT NULL, project_id INTEGER NOT NULL, branch_id INTEGER NOT NULL, file_name TEXT NOT NULL, hash TEXT NOT NULL);")
        .execute(conn)
        .await.unwrap();
    sqlx::query("INSERT INTO hashes (timestamp, project_id, branch_id, file_name, hash) VALUES (?, ?, ?, ?, ?)")
        .bind(get_timestamp())
        .bind("ac21edff")
        .bind("abd234d2")
        .bind(file_name)
        .bind(hash.to_string())
        .execute(conn)
        .await
        .unwrap();
}
