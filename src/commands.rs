use crate::{
    common::{Config, Project, Save, SavedFile},
    utils::{generate_id, get_timestamp},
};
use anyhow::Error;
use sqlx::{AssertSqlSafe, Pool, Sqlite};
use std::{
    env,
    fs::{self},
    path::Path,
};
use zstd::{
    bulk::{compress, decompress},
    compress_file,
    zstd_safe::WriteBuf,
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
        r#"INSERT INTO "saves_{}"
( id, timestamp, branch, comment)
VALUES ( ?, ?, ?, ?);"#,
        &project.id.as_ref().unwrap()
    );
    sqlx::query(AssertSqlSafe(insert_query))
        .bind(save_id)
        .bind(get_timestamp().to_string())
        .bind(&project.branch)
        .bind(comment)
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
        let absolute_path = path.canonicalize()?.to_string_lossy().to_string();

        if project.ignore.contains(&file_name) {
            continue;
        }
        if path.is_dir() {
            Box::pin(save_files(db, &path, comment, project, &save_id)).await?;
        } else {
            let file_size = fs::metadata(&path)?.len();
            if file_size > 1_000_000_000 {
                continue;
            }
            let file_content = fs::read(&path)?;
            let file_hash = blake3::hash(&file_content).to_hex().to_string();

            // CHECK IF FILE CHANGED
            let query = format!(
                r#"
                SELECT h.*
                FROM "hashes_{}" AS h
                JOIN "saves_{}" AS s
                    ON s.id = h.save_id
                WHERE h.hash = ?
                LIMIT 1
                "#,
                project.id.as_ref().unwrap(),
                project.id.as_ref().unwrap(),
            );

            let prev_save: Option<SavedFile> = sqlx::query_as(AssertSqlSafe(query))
                .bind(&file_hash)
                .fetch_optional(db)
                .await?;

            let location = prev_save.map(|save| save.id);

            let compressed_data: Option<Vec<u8>> = if location.is_none() {
                Some(compress(file_content.as_slice(), 3)?)
            } else {
                None
            };

            // INSERT NEW HASH
            let insert_query = format!(
                r#"INSERT INTO "hashes_{}"
        (id, save_id, file_name, hash, size, location, cdata)
        VALUES ( ?, ?, ?, ?, ?, ?, ?);"#,
                &project.id.as_ref().unwrap()
            );
            sqlx::query(AssertSqlSafe(insert_query))
                .bind(generate_id(8))
                .bind(save_id)
                .bind(absolute_path)
                .bind(file_hash)
                .bind(file_size.to_string())
                .bind(location)
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
        r#"SELECT file_name as name, hash, size, cdata FROM "hashes_{}" WHERE save_id = "{save_id}";"#,
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
        if file.name.contains("rec.toml") {
            continue;
        }

        let size: usize = file.size.parse()?;

        let data = if let Some(location) = &file.location {
            let query = format!(
                r#"
                SELECT h.*
                FROM "hashes_{}" AS h
                WHERE h.id = ?
                LIMIT 1
                "#,
                project.id.as_ref().unwrap(),
            );

            let prev_save: SavedFile = sqlx::query_as(AssertSqlSafe(query))
                .bind(location)
                .fetch_one(db)
                .await?;

            prev_save
                .cdata
                .ok_or_else(|| Error::msg("Missing compressed file data"))?
        } else {
            file.cdata
                .ok_or_else(|| Error::msg("Missing compressed file data"))?
        };

        let decompressed = decompress(&data, size)?;

        fs::write(&file.name, decompressed)
            .map_err(|e| anyhow::anyhow!("Failed to restore '{}': {}", file.name, e))?;
    }
    Ok(())
}

pub async fn log(db: &Pool<Sqlite>, project: &Project, limit: u32) -> Result<Vec<Save>, Error> {
    let query = format!(
        r#"SELECT * FROM "saves_{}" ORDER BY timestamp DESC LIMIT {limit};"#,
        &project.id.as_ref().unwrap(),
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
