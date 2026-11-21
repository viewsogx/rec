use anyhow::Error;
use blake3::hash;
use sqlx::{AssertSqlSafe, Pool, Sqlite};

use zstd::{bulk::decompress, compress_file, zstd_safe::WriteBuf};

use crate::{Config, Project, utils::generate_id};
use colored::*;
use std::{
    env,
    fs::{self, DirEntry},
    os::windows::fs::MetadataExt,
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
            let compressed_data = compress_file(&path, 3)?;
            let file_hash = blake3::hash(fs::read(&path)?.as_slice());
            let file_size = fs::metadata(&path)?.file_size();

            let insert_query = format!(
                "INSERT INTO 'hashes_{}'
        ( save_id, file_name, hash, size, cdata)
        VALUES ( ?, ?, ?, ?, ?)",
                &project.id.as_ref().unwrap()
            );
            sqlx::query(AssertSqlSafe(insert_query))
                .bind(&save_id)
                .bind(&path.to_str())
                .bind(file_hash.to_hex().to_string())
                .bind(file_size.to_string())
                .bind(compressed_data)
                .execute(db)
                .await?;
        }
    }

    Ok(())
}

pub fn init() -> Result<(), Error> {
    let default_name = env::current_dir()?
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("proj")
        .to_string();
    let project: Project = Project {
        name: default_name,
        branch: "main".into(),
        ignore: Vec::new(),
        remote: "https://recbase.live/[username]".into(),
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

pub async fn load(db:  &Pool<Sqlite>, save_id: &String, project: &Project) -> Result<(), Error> {
    println!("helo {}", save_id);
    sqlx::query("select filename, size, hash, cdata from 'hashes_?' inner join 'saves_?' on save_id = save_id").bind(project.id).bind(project.id).fetch_all(&db).await?;
    Ok(())
}
