use anyhow::Error;
use sqlx::{AssertSqlSafe, Pool, Sqlite};

use zstd::{bulk::decompress, compress_file};

use crate::{
    Config, Project,
    common::{Save, SavedFile},
    utils::{generate_id, get_timestamp},
};
use colored::*;
use std::{
    env,
    fs::{self},
    path::Path,
};

pub async fn save(
    db: &Pool<Sqlite>,
    dir: &Path,
    comment: &str,
    project: &Project,
    save_id: &String,
) -> Result<(), Error> {
    save_files(db, dir, comment, project, save_id).await?;
    let insert_query = format!(
        "INSERT INTO 'saves_{}'
( id, timestamp, branch, comment)
VALUES ( ?, ?, ?, ?)",
        &project.id.as_ref().unwrap()
    );
    sqlx::query(AssertSqlSafe(insert_query))
        .bind(&save_id)
        .bind(get_timestamp().to_string())
        .bind(&project.branch)
        .bind(&comment)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn save_files(
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
        let absolute_path = path.canonicalize()?;

        if project.ignore.contains(&file_name) {
            continue;
        }
        if path.is_dir() {
            Box::pin(save_files(db, &path, comment, project, &save_id)).await?;
        } else {
            let compressed_data = compress_file(&path, 3)?;
            let file_hash = blake3::hash(fs::read(&path)?.as_slice());
            let file_size = fs::metadata(&path)?.len();

            let insert_query = format!(
                "INSERT INTO 'hashes_{}'
        ( save_id, file_name, hash, size, cdata)
        VALUES ( ?, ?, ?, ?, ?)",
                &project.id.as_ref().unwrap()
            );
            sqlx::query(AssertSqlSafe(insert_query))
                .bind(&save_id)
                .bind(&absolute_path.to_string_lossy().to_string())
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
        .unwrap_or("untitled-project")
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
    Ok(())
}

pub async fn load(db: &Pool<Sqlite>, save_id: &String, project: &Project) -> Result<(), Error> {
    let query = format!(
        "SELECT file_name as name, hash, size, cdata FROM 'hashes_{}' WHERE save_id = '{save_id}'",
        &project.id.as_ref().unwrap(),
    );
    let Ok(files): Result<Vec<SavedFile>, _> =
        sqlx::query_as(AssertSqlSafe(query)).fetch_all(db).await
    else {
        return Err(Error::msg(format!(
            "Unexpected error occurred while loading save: {save_id}"
        )));
    };

    if files.len() == 0 {
        return Err(Error::msg("No matching save found"));
    }
    save(
        db,
        Path::new("."),
        &format!("<< Loaded save: {save_id}"),
        project,
        &generate_id(8),
    )
    .await?;
    for file in files {
        let size: usize = file.size.parse()?;
        if file.name.contains("rec.toml") {
            continue;
        }
        fs::write(&file.name, decompress(&file.cdata, size)?)
            .map_err(|e| anyhow::anyhow!("Failed to restore '{}': {}", file.name, e))?;
    }
    Ok(())
}

pub async fn log(db: &Pool<Sqlite>, project: &Project, limit: u32) -> Result<Vec<Save>, Error> {
    let project_id = &project.id.as_ref().unwrap();
    let query = format!(
        "SELECT * FROM 'saves_{}' ORDER BY timestamp DESC LIMIT {limit}",
        project_id,
    );
    let Ok(saves): Result<Vec<Save>, _> = sqlx::query_as(AssertSqlSafe(query)).fetch_all(db).await
    else {
        return Err(Error::msg(format!(
            "Unexpected error occurred while fetching saves for project: {}",
            project.name
        )));
    };
    Ok(saves)
}
