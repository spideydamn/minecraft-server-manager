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
            UNIQUE(profile_id, version_id)
        );
    ")?;

    // Migration: add server_dir column if it doesn't exist
    migrate_add_server_dir(&conn)?;

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

pub fn with_conn<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce(&Connection) -> anyhow::Result<T>,
{
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| anyhow::anyhow!("DB not initialized"))?;
    f(conn)
}
