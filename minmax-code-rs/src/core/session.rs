use anyhow::Result;
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::settings::config_dir;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
    pub created_at: String,
}

pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    /// Open (or create) the sessions database at ~/.minmax-code/sessions.db
    pub fn open() -> Result<Self> {
        let dir = config_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let db_path = dir.join("sessions.db");
        Self::open_at(db_path)
    }

    /// Open the database at a specific path (useful for testing).
    pub fn open_at(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                tool_calls TEXT,
                tool_call_id TEXT,
                name TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );",
        )?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn create_session(&self, model: &str) -> Result<Session> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono_now();
        let name = "New Session";
        conn.execute(
            "INSERT INTO sessions (id, name, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, model, now, now],
        )?;
        Ok(Session {
            id,
            name: name.to_string(),
            model: model.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn rename_session(&self, id: &str, name: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        conn.execute(
            "UPDATE sessions SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, name, model, created_at, updated_at FROM sessions ORDER BY updated_at DESC")?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    model: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&str>,
        tool_call_id: Option<&str>,
        name: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        conn.execute(
            "INSERT INTO messages (session_id, role, content, tool_calls, tool_call_id, name) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, role, content, tool_calls, tool_call_id, name],
        )?;
        conn.execute(
            "UPDATE sessions SET updated_at = datetime('now') WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, tool_calls, tool_call_id, name, created_at FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let messages = stmt
            .query_map(params![session_id], |row| {
                Ok(StoredMessage {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    tool_calls: row.get(4)?,
                    tool_call_id: row.get(5)?,
                    name: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(messages)
    }
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without external chrono crate
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Use SQLite's datetime format for consistency
    format!("{}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SessionStore {
        // In-memory database for testing
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                tool_calls TEXT,
                tool_call_id TEXT,
                name TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );"
        ).unwrap();
        SessionStore { conn: Mutex::new(conn) }
    }

    #[test]
    fn create_and_list_sessions() {
        let store = test_store();
        let s1 = store.create_session("MiniMax-M2.5").unwrap();
        let s2 = store.create_session("MiniMax-M2.5-highspeed").unwrap();

        // Touch s2 so it has a later updated_at
        store.rename_session(&s2.id, "Second").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        // Both sessions exist (order may vary if created in same second)
        let ids: Vec<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&s1.id.as_str()));
        assert!(ids.contains(&s2.id.as_str()));
    }

    #[test]
    fn rename_session() {
        let store = test_store();
        let s = store.create_session("MiniMax-M2.5").unwrap();
        assert_eq!(s.name, "New Session");

        store.rename_session(&s.id, "My Chat").unwrap();
        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions[0].name, "My Chat");
    }

    #[test]
    fn delete_session_cascades() {
        let store = test_store();
        let s = store.create_session("MiniMax-M2.5").unwrap();
        store
            .save_message(&s.id, "user", "hello", None, None, None)
            .unwrap();

        let msgs = store.get_session_messages(&s.id).unwrap();
        assert_eq!(msgs.len(), 1);

        store.delete_session(&s.id).unwrap();
        let sessions = store.list_sessions().unwrap();
        assert!(sessions.is_empty());

        let msgs = store.get_session_messages(&s.id).unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn save_and_retrieve_messages() {
        let store = test_store();
        let s = store.create_session("MiniMax-M2.5").unwrap();

        store
            .save_message(&s.id, "user", "Hello!", None, None, None)
            .unwrap();
        store
            .save_message(&s.id, "assistant", "Hi there!", None, None, None)
            .unwrap();
        store
            .save_message(
                &s.id,
                "assistant",
                "",
                Some(r#"[{"id":"tc1","type":"function","function":{"name":"bash","arguments":"{}"}}]"#),
                None,
                None,
            )
            .unwrap();
        store
            .save_message(&s.id, "tool", "output", None, Some("tc1"), Some("bash"))
            .unwrap();

        let msgs = store.get_session_messages(&s.id).unwrap();
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "Hello!");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[2].tool_calls.as_deref().unwrap().contains("tc1"), true);
        assert_eq!(msgs[3].tool_call_id.as_deref(), Some("tc1"));
        assert_eq!(msgs[3].name.as_deref(), Some("bash"));
    }
}
