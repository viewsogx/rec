use anyhow::Error;
use sqlx::{AssertSqlSafe, Pool, Sqlite};

use crate::{
    Project,
    utils::{generate_id, get_timestamp},
};

use std::{
    fs::{self, DirEntry},
    path::{Path, PathBuf},
};

pub async fn save(
    dir: &Path,
    db: &Pool<Sqlite>,
    comment: &str,
    project: &Project,
) -> Result<(), Error> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name: Vec<String> = path
                .to_str()
                .unwrap()
                .to_string()
                .split(r#"\"#)
                .map(|s| s.to_string())
                .collect();

            if project.ignore.contains(&file_name[1]) {
                continue;
            }
            if path.is_dir() {
                Box::pin(save(&path, db, comment, project)).await?;
            } else {
                let hash = blake3::hash(fs::read(&path)?.as_slice());
                let insert_query = format!(
                    "INSERT INTO '{}'
        (id, timestamp, branch, file_name, hash, comment)
        VALUES (?, ?, ?, ?, ?, ?)",
                    &project.name
                );
                sqlx::query(AssertSqlSafe(insert_query))
                    .bind(generate_id(12))
                    .bind(get_timestamp())
                    .bind(&project.branch)
                    .bind(&path.to_str())
                    .bind(hash.to_hex().to_string())
                    .bind(comment)
                    .execute(db)
                    .await?;
            }
        }
    }

    Ok(())
}
