use sqlx::{Pool, Sqlite};

use crate::{
    Project,
    utils::{generate_id, get_timestamp},
};

use std::path::PathBuf;
use tokio::fs;

pub async fn save(
    db: &Pool<Sqlite>,
    comment: &str,
    project: &Project,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut dirs = vec![PathBuf::from(".")];

    while let Some(dir) = dirs.pop() {
        let mut entries = match fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Cannot access {:?}: {}", dir, e);
                continue;
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Cannot inspect {:?}: {}", path, e);
                    continue;
                }
            };

            println!("{:?}", dir);
            // Directory: add it to the traversal stack.
            if file_type.is_dir() {
                dirs.push(path);
                continue;
            }

            // Skip anything that isn't a regular file.
            if !file_type.is_file() {
                continue;
            }

            // TODO: Check ignore rules here.
            let dirname: Vec<&str> = dir.to_str().unwrap().split(".\\").collect();
            if project.ignore.contains(dirname[1]) {
                continue;
            }
            // Read file.
            let file = match fs::read(&path).await {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Cannot read {:?}: {}", path, e);
                    continue;
                }
            };

            // Generate BLAKE3 hash.
            let hash = blake3::hash(&file);

            let file_name = path.to_string_lossy().into_owned();

            // Save to database.
            sqlx::query(
                "INSERT INTO hashes
                (id, timestamp, project, branch, file_name, hash, comment)
                VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(generate_id(12))
            .bind(get_timestamp())
            .bind(&project.name)
            .bind(&project.branch)
            .bind(&file_name)
            .bind(hash.to_hex().to_string())
            .bind(comment)
            .execute(db)
            .await?;
        }
    }

    Ok(())
}
