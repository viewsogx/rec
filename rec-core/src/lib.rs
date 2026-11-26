pub mod models;
pub mod utils;

use anyhow::{Error, anyhow};
use models::{Config, File, Project, Save, ShortFile};
use rusqlite::{Connection, params};
use rusqlite_from_row::FromRow;
use std::{
    env,
    fs::{self},
    path::Path,
};
use utils::{generate_id, get_timestamp};
use zstd::bulk::{compress, decompress};

/// Saves the current working directory and returns the files that were too
/// large to be stored.
pub fn save(db: &Connection, comment: &str, project: &Project) -> Result<Vec<ShortFile>, Error> {
    let save_id = generate_id(8);
    let large_files = save_files(db, Path::new("."), project, &save_id)?;
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

    Ok(large_files)
}

fn save_files(
    db: &Connection,
    dir: &Path,
    project: &Project,
    save_id: &str,
) -> Result<Vec<ShortFile>, Error> {
    let mut large_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let absolute_path = path.canonicalize()?.to_string_lossy().to_string();

        if is_ignored(project, &file_name) {
            continue;
        }
        if path.is_dir() {
            large_files.extend(save_files(db, &path, project, save_id)?);
        } else {
            let file_size = fs::metadata(&path)?.len();
            if file_size > 2_100_000_000 {
                large_files.push(ShortFile {
                    file_name,
                    size: file_size,
                });
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

            let data_dir = dirs::data_dir()
                .expect("no data dir")
                .join(".rec")
                .join("data");
            fs::create_dir_all(&data_dir)?;
            let data_dir = data_dir.join(&file_hash);

            if !fs::exists(&data_dir)? {
                let compressed_data: Vec<u8> =
                    compress(file_content.as_slice(), compression_level)?;
                fs::write(data_dir, compressed_data)?
            }

            // INSERT NEW HASH
            let insert_query = format!(
                r#"INSERT INTO "hashes_{}"
        (id, save_id, file_name, hash, size)
        VALUES ( ?, ?, ?, ?, ?);"#,
                project.id.as_ref().unwrap()
            );
            db.execute(
                &insert_query,
                params![
                    generate_id(8),
                    save_id,
                    absolute_path,
                    file_hash,
                    file_size.to_string()
                ],
            )?;
        }
    }

    Ok(large_files)
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
    let Ok(files): Result<Vec<File>, _> = db
        .prepare(&query)?
        .query_map([], File::try_from_row)?
        .collect()
    else {
        return Err(Error::msg(format!(
            "Unexpected error occurred while loading save: {save_id}"
        )));
    };

    if files.len() == 0 {
        return Err(Error::msg("No matching save found"));
    }
    save(db, &format!("<< Loaded save: {save_id} >>"), project)?;
    let mut saved_paths: Vec<&String> = Vec::new();
    for file in &files {
        saved_paths.push(&file.file_name);
        let file_path = Path::new(&file.file_name);
        let file_name = &file_path.file_name().unwrap().to_string_lossy().to_string();
        if file_name == "rec.toml" || is_ignored(project, file_name) {
            continue;
        }

        let size: usize = file.size.parse()?;

        let data_dir = dirs::data_dir()
            .expect("no data dir")
            .join(".rec")
            .join("data")
            .join(&file.hash);
        let Ok(data) = fs::read(data_dir) else {
            return Err(anyhow!("No save found for file: {}", file.file_name));
        };
        let decompressed = decompress(&data, size)?;
        if !fs::exists(&file.file_name)? {
            let parent = file_path.parent().unwrap();
            let parent_string = parent.to_string_lossy().to_string();
            fs::create_dir_all(parent_string)?;
        }
        fs::write(&file.file_name, decompressed)
            .map_err(|e| anyhow!("Failed to restore '{}': {}", file.file_name, e))?;
    }

    delete_not_saved(Path::new("."), project, &saved_paths)?;
    Ok(())
}

fn delete_not_saved(path: &Path, project: &Project, saved_paths: &[&String]) -> Result<(), Error> {
    // Collect the entries before deleting any of them. Removing entries while a
    // directory is still being iterated can cause siblings to be skipped.
    let entries: Vec<_> = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()?;

    for entry in entries {
        let file_name = entry.file_name().unwrap().to_string_lossy().to_string();
        if is_ignored(project, &file_name) {
            continue;
        }

        if entry.is_dir() {
            delete_not_saved(&entry, project, saved_paths)?;
            // Saves only store files, so a directory that is empty once its
            // unsaved contents are gone is not part of the save. This also
            // removes directories that were already empty before loading.
            if fs::read_dir(&entry)?.next().is_none() {
                fs::remove_dir(&entry)?;
            }
            continue;
        }

        let Ok(existing_path) = entry.canonicalize() else {
            continue;
        };
        let existing_path = existing_path.to_string_lossy().to_string();
        if saved_paths
            .iter()
            .any(|saved_path| *saved_path == &existing_path)
        {
            continue;
        }

        fs::remove_file(&entry)?;
    }

    Ok(())
}

fn is_ignored(project: &Project, path: &String) -> bool {
    if project.ignore.contains(path) {
        return true;
    }
    false
}

pub fn list(db: &Connection, project: &Project, limit: u32) -> Result<Vec<Save>, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_project(ignore: &[&str]) -> Project {
        Project {
            name: "test".into(),
            branch: "main".into(),
            ignore: ignore.iter().map(|name| name.to_string()).collect(),
            remote: String::new(),
            id: Some("test".into()),
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("rec-{name}-{}", generate_id(8)));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn deletes_unsaved_files_and_directories() -> Result<(), Error> {
        let root = test_dir("delete-unsaved");

        fs::write(root.join("saved.txt"), "saved")?;
        fs::write(root.join("unsaved.txt"), "unsaved")?;

        fs::create_dir_all(root.join("saved_dir"))?;
        fs::write(root.join("saved_dir").join("keep.txt"), "keep")?;
        fs::create_dir_all(root.join("saved_dir").join("empty_subdir"))?;

        fs::create_dir_all(root.join("empty_dir"))?;
        fs::create_dir_all(root.join("unsaved_dir").join("nested"))?;
        fs::write(
            root.join("unsaved_dir").join("nested").join("drop.txt"),
            "drop",
        )?;

        fs::create_dir_all(root.join("ignored_dir"))?;
        fs::write(root.join("ignored_dir").join("keep.txt"), "keep")?;

        let saved_paths: Vec<String> = [
            root.join("saved.txt"),
            root.join("saved_dir").join("keep.txt"),
        ]
        .iter()
        .map(|path| path.canonicalize().unwrap().to_string_lossy().to_string())
        .collect();
        let saved_path_refs: Vec<&String> = saved_paths.iter().collect();

        let project = test_project(&["ignored_dir"]);
        delete_not_saved(&root, &project, &saved_path_refs)?;

        assert!(root.join("saved.txt").is_file());
        assert!(root.join("saved_dir").join("keep.txt").is_file());
        assert!(root.join("ignored_dir").join("keep.txt").is_file());

        assert!(!root.join("saved_dir").join("empty_subdir").exists());
        assert!(!root.join("empty_dir").exists());
        assert!(!root.join("unsaved_dir").exists());
        assert!(!root.join("unsaved.txt").exists());

        fs::remove_dir_all(&root)?;
        Ok(())
    }
}
