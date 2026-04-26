use rusqlite::{Connection, Result, params};
use std::sync::Mutex;

static DB: Mutex<Option<Connection>> = Mutex::new(None);

pub fn init(path: &str) -> Result<()> {
    let conn = Connection::open(path)?;

    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS connection_profiles (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            host        TEXT NOT NULL,
            port        INTEGER NOT NULL DEFAULT 22,
            username    TEXT NOT NULL,
            auth_method TEXT NOT NULL CHECK(auth_method IN ('password','key')),
            password    TEXT,
            key_path    TEXT,
            server_dir  TEXT NOT NULL DEFAULT '~/minecraft',
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS minecraft_versions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_id  INTEGER NOT NULL REFERENCES connection_profiles(id) ON DELETE CASCADE,
            version_id  TEXT NOT NULL,
            jar_name    TEXT NOT NULL,
            server_dir  TEXT NOT NULL DEFAULT '~/minecraft',
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            in_use      INTEGER NOT NULL DEFAULT 0,
            installation_date TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(profile_id, version_id)
        );
    ")?;

    // Migration: add server_dir column if it doesn't exist
    migrate_add_server_dir(&conn)?;

    // Migration: add in_use and installation_date columns if they don't exist
    migrate_version_management_fields(&conn)?;

    *DB.lock().unwrap() = Some(conn);
    Ok(())
}

fn migrate_add_server_dir(conn: &Connection) -> Result<()> {
    // Check if server_dir column exists in connection_profiles
    let has_column: Result<bool> = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('connection_profiles') WHERE name = 'server_dir'",
        [],
        |row| row.get(0),
    );

    if let Ok(false) = has_column {
        conn.execute(
            "ALTER TABLE connection_profiles ADD COLUMN server_dir TEXT NOT NULL DEFAULT '~/minecraft'",
            [],
        )?;
    }

    Ok(())
}

fn migrate_version_management_fields(conn: &Connection) -> Result<()> {
    // Check if in_use column exists in minecraft_versions
    let has_in_use: Result<bool> = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('minecraft_versions') WHERE name = 'in_use'",
        [],
        |row| row.get(0),
    );

    if let Ok(false) = has_in_use {
        conn.execute(
            "ALTER TABLE minecraft_versions ADD COLUMN in_use INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    // Check if installation_date column exists in minecraft_versions
    let has_installation_date: Result<bool> = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('minecraft_versions') WHERE name = 'installation_date'",
        [],
        |row| row.get(0),
    );

    if let Ok(false) = has_installation_date {
        // SQLite doesn't support non-constant defaults in ALTER TABLE
        // Add column with a constant default, then update existing rows
        conn.execute(
            "ALTER TABLE minecraft_versions ADD COLUMN installation_date TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }

    // Always update records with empty installation_date (handles both new column and existing empty values)
    conn.execute(
        "UPDATE minecraft_versions SET installation_date = datetime('now') WHERE installation_date = '' OR installation_date IS NULL",
        [],
    )?;

    // Fix records where jar_name contains a directory path instead of jar filename
    // This happens when jar_name was incorrectly saved as the version directory path
    // Use raw version_id to match the installer format (minecraft_server_{version_id}.jar)
    conn.execute(
        "UPDATE minecraft_versions
         SET jar_name = 'minecraft_server_' || version_id || '.jar',
             server_dir = jar_name
         WHERE jar_name LIKE '%minecraft-server-%'
           AND jar_name NOT LIKE '%.jar'",
        [],
    )?;

    // Clean up invalid records (those without jar_name)
    conn.execute(
        "DELETE FROM minecraft_versions WHERE jar_name IS NULL OR jar_name = ''",
        [],
    )?;

    Ok(())
}

pub fn with_conn<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce(&Connection) -> anyhow::Result<T>,
{
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| anyhow::anyhow!("DB not initialized"))?;
    f(conn)
}

/// Check if a version is in use by any active servers
pub fn is_version_in_use(profile_id: i64, version_id: &str) -> anyhow::Result<bool> {
    with_conn(|conn| {
        let in_use: i64 = match conn.query_row(
            "SELECT in_use FROM minecraft_versions WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
            |row| row.get(0),
        ) {
            Ok(val) => val,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };
        Ok(in_use == 1)
    })
}

/// Update version in-use status
pub fn set_version_in_use(profile_id: i64, version_id: &str, in_use: bool) -> anyhow::Result<()> {
    with_conn(|conn| {
        conn.execute(
            "UPDATE minecraft_versions SET in_use = ?1 WHERE profile_id = ?2 AND version_id = ?3",
            rusqlite::params![if in_use { 1 } else { 0 }, profile_id, version_id],
        )?;
        Ok(())
    })
}

/// Get servers using a specific version (for error messages)
pub fn get_servers_using_version(profile_id: i64, version_id: &str) -> anyhow::Result<Vec<String>> {
    with_conn(|conn| {
        // Check in_use status directly to avoid nested with_conn deadlock
        let in_use: i64 = match conn.query_row(
            "SELECT in_use FROM minecraft_versions WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
            |row| row.get(0),
        ) {
            Ok(val) => val,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };

        if in_use != 1 {
            return Ok(vec![]);
        }

        let mut stmt = conn.prepare(
            "SELECT name FROM connection_profiles WHERE id = ?1"
        )?;
        let profile_name: String = match stmt.query_row(rusqlite::params![profile_id], |row| {
            row.get(0)
        }) {
            Ok(name) => name,
            Err(rusqlite::Error::QueryReturnedNoRows) => format!("Profile {}", profile_id),
            Err(e) => return Err(e.into()),
        };

        Ok(vec![profile_name])
    })
}
