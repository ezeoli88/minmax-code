import { Database } from "bun:sqlite";
import { existsSync, mkdirSync } from "fs";
import { homedir } from "os";
import { join } from "path";

export interface Session {
  id: string;
  name: string;
  model: string;
  created_at: string;
  updated_at: string;
}

export interface StoredMessage {
  id: number;
  session_id: string;
  role: string;
  content: string;
  tool_calls: string | null;
  tool_call_id: string | null;
  name: string | null;
  created_at: string;
}

const DB_DIR = join(homedir(), ".minmax-terminal");
const DB_PATH = join(DB_DIR, "sessions.db");

let db: Database | null = null;

export function getDb(): Database {
  if (db) return db;

  if (!existsSync(DB_DIR)) {
    mkdirSync(DB_DIR, { recursive: true });
  }

  db = new Database(DB_PATH);
  db.run("PRAGMA journal_mode = WAL");
  db.run("PRAGMA foreign_keys = ON");

  db.run(`
    CREATE TABLE IF NOT EXISTS sessions (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      model TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT (datetime('now')),
      updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    )
  `);

  db.run(`
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
    )
  `);

  return db;
}

export function createSession(model: string): Session {
  const id = crypto.randomUUID();
  const now = new Date().toISOString();
  const name = "New Session";
  getDb().run(
    "INSERT INTO sessions (id, name, model, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    [id, name, model, now, now]
  );
  return { id, name, model, created_at: now, updated_at: now };
}

export function renameSession(id: string, name: string): void {
  getDb().run("UPDATE sessions SET name = ?, updated_at = datetime('now') WHERE id = ?", [
    name,
    id,
  ]);
}

export function listSessions(): Session[] {
  return getDb()
    .query("SELECT * FROM sessions ORDER BY updated_at DESC")
    .all() as Session[];
}

export function deleteSession(id: string): void {
  getDb().run("DELETE FROM sessions WHERE id = ?", [id]);
}

export function saveMessage(
  sessionId: string,
  role: string,
  content: string,
  toolCalls?: any[] | null,
  toolCallId?: string | null,
  name?: string | null
): void {
  getDb().run(
    "INSERT INTO messages (session_id, role, content, tool_calls, tool_call_id, name) VALUES (?, ?, ?, ?, ?, ?)",
    [
      sessionId,
      role,
      content,
      toolCalls ? JSON.stringify(toolCalls) : null,
      toolCallId || null,
      name || null,
    ]
  );
  getDb().run("UPDATE sessions SET updated_at = datetime('now') WHERE id = ?", [sessionId]);
}

export function getSessionMessages(sessionId: string): StoredMessage[] {
  return getDb()
    .query("SELECT * FROM messages WHERE session_id = ? ORDER BY id ASC")
    .all(sessionId) as StoredMessage[];
}

export function closeDb(): void {
  if (db) {
    db.close();
    db = null;
  }
}
