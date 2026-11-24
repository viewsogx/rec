use crate::{
    common::{Config, Project, Save, SavedFile},
    utils::{generate_id, get_timestamp},
};
use anyhow::Error;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_from_row::FromRow;
use std::{
    env,
    fs::{self},
    path::Path,
};
use zstd::bulk::{compress, decompress};

pub fn save(
    db: &Connection,
    dir: &Path,
    comment: &str,
    project: &Project,
    save_id: &String,
) -> Result<(), Error> {
    save_files(db, dir, comment, project, save_id)?;
    let insert_query = format!(
        r#"INSERT INTO "saves_{}"
( id, timestamp, branch, comment)
VALUES ( ?, ?, ?, ?);"#,
        project.id.as_ref().unwrap()
    );
    db.execute(
        &insert_query,
        params![
            save_id,
            get_timestamp().to_string(),
            &project.branch,
            comment
        ],
    )?;

    Ok(())
}

pub fn save_files(
    db: &Connection,
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
            save_files(db, &path, comment, project, &save_id)?;
        } else {
            let file_size = fs::metadata(&path)?.len();
            if file_size > 2_100_000_000 {
                continue;
            }
            let compression_level = if file_size > 500_000_000 {
                12
            } else if file_size > 4_000_000 {
                7
            } else {
                3
            };
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
            let prev_save: Option<SavedFile> = db
                .query_row(&query, params![&file_hash], SavedFile::try_from_row)
                .optional()
                .unwrap();

            let location = prev_save.map(|save| save.id);

            let compressed_data: Option<Vec<u8>> = if location.is_none() {
                Some(compress(file_content.as_slice(), compression_level)?)
            } else {
                None
            };

            // INSERT NEW HASH
            let insert_query = format!(
                r#"INSERT INTO "hashes_{}"
        (id, save_id, file_name, hash, size, location, cdata)
        VALUES ( ?, ?, ?, ?, ?, ?, ?);"#,
                project.id.as_ref().unwrap()
            );
            db.execute(
                &insert_query,
                params![
                    generate_id(8),
                    save_id,
                    absolute_path,
                    file_hash,
                    file_size.to_string(),
                    location,
                    compressed_data
                ],
            )?;
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

pub fn load(db: &Connection, save_id: &String, project: &Project) -> Result<(), Error> {
    let query = format!(
        r#"SELECT * FROM "hashes_{}" WHERE save_id = "{save_id}";"#,
        project.id.as_ref().unwrap(),
    );
    let Ok(files): Result<Vec<SavedFile>, _> = db
        .prepare(&query)?
        .query_map([], SavedFile::try_from_row)?
        .collect()
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
    )?;
    for file in files {
        if file.file_name.contains("rec.toml") {
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

            let prev_save: SavedFile = db.query_row(&query, [location], SavedFile::try_from_row)?;

            prev_save
                .cdata
                .ok_or_else(|| Error::msg("Missing compressed file data"))?
        } else {
            file.cdata
                .ok_or_else(|| Error::msg("Missing compressed file data"))?
        };

        let decompressed = decompress(&data, size)?;

        fs::write(&file.file_name, decompressed)
            .map_err(|e| anyhow::anyhow!("Failed to restore '{}': {}", file.file_name, e))?;
    }
    Ok(())
}

pub fn log(db: &Connection, project: &Project, limit: u32) -> Result<Vec<Save>, Error> {
    let query = format!(
        r#"SELECT * FROM "saves_{}" ORDER BY timestamp DESC LIMIT {limit};"#,
        project.id.as_ref().unwrap(),
    );
    let Ok(saves): Result<Vec<Save>, _> = db
        .prepare(&query)?
        .query_map([], Save::try_from_row)?
        .collect()
    else {
        return Err(Error::msg(format!(
            "Unexpected error occurred while fetching saves for project: {}",
            project.name
        )));
    };
    Ok(saves)
}
