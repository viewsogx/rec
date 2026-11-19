use anyhow::Error;
use sqlx::{AssertSqlSafe, Pool, Sqlite};

use crate::{Config, Project, utils::generate_id};
use colored::*;
use std::{
    env,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
};

pub async fn save(
    db: &Pool<Sqlite>,
    dir: &Path,
    comment: &str,
    project: &Project,
    save_id: &String,
) -> Result<(), Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if project.ignore.contains(&file_name) {
            continue;
        }
        if path.is_dir() {
            Box::pin(save(db, &path, comment, project, &save_id)).await?;
        } else {
            let hash = blake3::hash(fs::read(&path)?.as_slice());
            let insert_query = format!(
                "INSERT INTO 'hashes_{}'
        ( save_id, file_name, hash)
        VALUES ( ?, ?, ?)",
                &project.id.as_ref().unwrap()
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

pub fn init() -> Result<(), Error> {
    let default_name = env::current_dir()?
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let project: Project = Project {
        name: default_name,
        branch: "main".into(),
        ignore: Vec::new(),
        remote: "https://recbase.org/[username]".into(),
        id: Some(generate_id(8)),
    };
    let config = Config { config: project };
    fs::write("rec.toml", toml::to_string_pretty(&config)?)?;
    println!(
        "{}",
        "Successfully initialized rec project (rec.toml created)".bright_green()
    );
    Ok(())
}
