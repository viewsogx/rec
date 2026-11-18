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
    save_id: &String,
) -> Result<(), Error> {
    // TODO INSERT ONCE INTO SAVES_SOME-PROJECT TABLE
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if project.ignore.contains(&file_name) {
            continue;
        }
        if path.is_dir() {
            Box::pin(save(&path, db, comment, project, &save_id)).await?;
        } else {
            let hash = blake3::hash(fs::read(&path)?.as_slice());
            let insert_query = format!(
                "INSERT INTO 'hashes_{}'
        ( save_id, file_name, hash)
        VALUES ( ?, ?, ?)",
                &project.name
            );
            sqlx::query(AssertSqlSafe(insert_query))
                .bind(&save_id)
                .bind(&path.to_str())
                .bind(hash.to_hex().to_string())
                .execute(db)
                .await?;
        }
    }

    Ok(())
}
