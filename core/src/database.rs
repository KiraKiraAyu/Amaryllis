use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
};

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};

pub use sea_orm::DbErr;

use crate::error::{AppError, Result};

pub async fn init_database(db_url: &str) -> Result<DatabaseConnection> {
    ensure_sqlite_file_ready(db_url)?;

    let mut options = ConnectOptions::new(db_url.to_string());
    options.sqlx_logging(false);

    if is_memory_db(db_url) {
        options.max_connections(1).min_connections(1);
    } else {
        options.max_connections(10).min_connections(1);
    }

    let db = Database::connect(options).await?;

    db.execute_unprepared("PRAGMA foreign_keys = ON;").await?;
    if !is_memory_db(db_url) {
        db.execute_unprepared("PRAGMA journal_mode = WAL;").await?;
        db.execute_unprepared("PRAGMA synchronous = NORMAL;")
            .await?;
    }

    Migrator::up(&db, None).await?;

    Ok(db)
}

fn ensure_sqlite_file_ready(db_url: &str) -> Result<()> {
    if is_memory_db(db_url) {
        return Ok(());
    }

    let Some(path) = sqlite_file_path(db_url) else {
        return Err(AppError::Internal(format!(
            "unsupported sqlite url format: {db_url}"
        )));
    };

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|err| AppError::Internal(format!("Database I/O error: {err}")))?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| AppError::Internal(format!("Database I/O error: {err}")))?;

    Ok(())
}

fn is_memory_db(db_url: &str) -> bool {
    db_url == "sqlite::memory:" || db_url.ends_with(":memory:")
}

fn sqlite_file_path(db_url: &str) -> Option<PathBuf> {
    let raw = db_url
        .strip_prefix("sqlite://")
        .or_else(|| db_url.strip_prefix("sqlite:"))?;

    let cleaned = raw.split('?').next().unwrap_or(raw);
    if cleaned.is_empty() {
        return None;
    }

    Some(PathBuf::from(cleaned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn sqlite_file_path_preserves_absolute_paths() {
        let path =
            sqlite_file_path("sqlite:///tmp/quantaura.db?mode=rwc").expect("absolute sqlite path");

        assert_eq!(path, PathBuf::from("/tmp/quantaura.db"));
    }

    #[test]
    fn ensure_sqlite_file_ready_creates_missing_db_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("quantaura-db-{unique}"));
        let file = dir.join("test.db");
        let url = format!("sqlite://{}", file.display());

        ensure_sqlite_file_ready(&url).expect("prepare sqlite file");

        assert!(file.exists(), "sqlite database file should be created");

        fs::remove_file(&file).expect("remove sqlite test file");
        fs::remove_dir_all(&dir).expect("remove sqlite test dir");
    }
}
