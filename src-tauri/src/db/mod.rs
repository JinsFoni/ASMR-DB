pub mod models;
pub mod queries;

use rusqlite::Connection;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 获取数据库路径：优先存放在软件可执行文件同级目录下（便携独立模式），
/// 若旧版本 %APPDATA% 目录下存在数据库而同级目录不存在，会自动无缝迁移。
pub fn get_db_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 1. 优先使用当前可执行程序所在目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let local_db = exe_dir.join("dlsite_manager.db");

            // 如果同级数据库不存在，但旧的 app_data_dir 存在数据库，自动迁移复制过来
            if !local_db.exists() {
                if let Ok(app_dir) = app.path().app_data_dir() {
                    let old_db = app_dir.join("dlsite_manager.db");
                    if old_db.exists() {
                        let _ = std::fs::copy(&old_db, &local_db);
                        let _ = std::fs::copy(
                            app_dir.join("dlsite_manager.db-wal"),
                            exe_dir.join("dlsite_manager.db-wal"),
                        );
                        let _ = std::fs::copy(
                            app_dir.join("dlsite_manager.db-shm"),
                            exe_dir.join("dlsite_manager.db-shm"),
                        );
                    }
                }
            }

            // 测试同级目录是否可写
            if let Ok(f) = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&local_db)
            {
                drop(f);
                return Ok(local_db);
            }
        }
    }

    // 2. 兜底回退：如果可执行目录受权限限制无法写入，则使用系统 AppData
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("dlsite_manager.db"))
}

/// Open (or create) the SQLite database and run migrations.
pub fn init_database(app: &AppHandle) -> Result<Connection, Box<dyn std::error::Error>> {
    let db_path = get_db_path(app)?;
    let conn = Connection::open(&db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Create all tables / indexes if they do not exist yet.
fn migrate(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        r#"
        -- ==================== 作品表 ====================
        CREATE TABLE IF NOT EXISTS works (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            rj_code         TEXT    NOT NULL UNIQUE,
            title_ja        TEXT,
            title_zh        TEXT,
            title_en        TEXT,
            circle_name     TEXT,
            circle_id       TEXT,
            cover_url       TEXT,
            work_type       TEXT    NOT NULL DEFAULT 'voice',
            price           INTEGER,
            sale_date       TEXT,
            description     TEXT,
            age_class       TEXT,
            duration_min    INTEGER,
            file_size_mb    REAL,
            dlsite_url      TEXT,
            created_at      TEXT    DEFAULT (datetime('now')),
            updated_at      TEXT    DEFAULT (datetime('now'))
        );

        -- ==================== 声优表 ====================
        CREATE TABLE IF NOT EXISTS voice_actors (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT    NOT NULL UNIQUE,
            dlsite_url      TEXT
        );

        -- ==================== 作品-声优关联表 ====================
        CREATE TABLE IF NOT EXISTS work_actors (
            work_id         INTEGER NOT NULL,
            actor_id        INTEGER NOT NULL,
            PRIMARY KEY (work_id, actor_id),
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
            FOREIGN KEY (actor_id) REFERENCES voice_actors(id) ON DELETE CASCADE
        );

        -- ==================== 作品标签表 ====================
        CREATE TABLE IF NOT EXISTS tags (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT    NOT NULL UNIQUE,
            color           TEXT    DEFAULT '#888888',
            is_preset       INTEGER DEFAULT 0
        );

        -- ==================== 作品-标签关联表 ====================
        CREATE TABLE IF NOT EXISTS work_tags (
            work_id         INTEGER NOT NULL,
            tag_id          INTEGER NOT NULL,
            is_auto         INTEGER DEFAULT 0,
            PRIMARY KEY (work_id, tag_id),
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        -- ==================== 分组表 ====================
        CREATE TABLE IF NOT EXISTS groups (
            id     INTEGER PRIMARY KEY AUTOINCREMENT,
            name   TEXT    NOT NULL UNIQUE,
            color  TEXT    DEFAULT '#597ef7'
        );

        -- ==================== 作品-分组关联表（一个作品只属于一个分组） ====================
        CREATE TABLE IF NOT EXISTS work_groups (
            work_id  INTEGER PRIMARY KEY,
            group_id INTEGER NOT NULL,
            FOREIGN KEY (work_id)  REFERENCES works(id)  ON DELETE CASCADE,
            FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
        );

        -- ==================== 账号表 ====================
        CREATE TABLE IF NOT EXISTS accounts (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT    NOT NULL,
            account_type    TEXT    DEFAULT 'dlsite',
            created_at      TEXT    DEFAULT (datetime('now'))
        );

        -- ==================== 作品-账号关联表 ====================
        CREATE TABLE IF NOT EXISTS work_accounts (
            work_id         INTEGER NOT NULL,
            account_id      INTEGER NOT NULL,
            purchased_at    TEXT,
            PRIMARY KEY (work_id, account_id),
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
            FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
        );

        -- ==================== 本地文件表 ====================
        CREATE TABLE IF NOT EXISTS local_files (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            work_id         INTEGER NOT NULL,
            local_path      TEXT    NOT NULL,
            file_type       TEXT    DEFAULT 'archive',
            download_status TEXT    DEFAULT 'not_downloaded',
            download_progress REAL   DEFAULT 0,
            file_size       INTEGER,
            created_at      TEXT    DEFAULT (datetime('now')),
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
            UNIQUE (work_id, local_path)
        );

        -- ==================== 音频文件表 ====================
        CREATE TABLE IF NOT EXISTS audio_tracks (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            work_id         INTEGER NOT NULL,
            file_path       TEXT    NOT NULL,
            track_number    INTEGER,
            title           TEXT,
            duration_sec    REAL,
            file_format     TEXT,
            file_size       INTEGER,
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE
        );

        -- ==================== 播放记录表 ====================
        CREATE TABLE IF NOT EXISTS play_history (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            work_id         INTEGER NOT NULL,
            track_id        INTEGER,
            last_position   REAL    DEFAULT 0,
            play_count      INTEGER DEFAULT 0,
            last_played_at  TEXT,
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
            FOREIGN KEY (track_id) REFERENCES audio_tracks(id) ON DELETE SET NULL
        );

        -- ==================== 设置表 ====================
        CREATE TABLE IF NOT EXISTS settings (
            key             TEXT PRIMARY KEY,
            value           TEXT
        );

        -- ==================== 索引 ====================
        CREATE INDEX IF NOT EXISTS idx_works_rj ON works(rj_code);
        CREATE INDEX IF NOT EXISTS idx_works_circle ON works(circle_name);
        CREATE INDEX IF NOT EXISTS idx_works_type ON works(work_type);
        CREATE INDEX IF NOT EXISTS idx_audio_work ON audio_tracks(work_id);
        CREATE INDEX IF NOT EXISTS idx_play_history_work ON play_history(work_id);
        CREATE INDEX IF NOT EXISTS idx_play_history_played ON play_history(last_played_at);

        -- ==================== 全文搜索虚拟表 ====================
        -- 使用独立 FTS5 表（不绑定 content='works'），避免外部内容表在
        -- 删除/重建索引时出现 "database disk image is malformed"。
        -- 旧库若为 external content 表则先删除重建。
        DROP TABLE IF EXISTS works_fts;
        CREATE VIRTUAL TABLE IF NOT EXISTS works_fts USING fts5(
            rj_code, title_ja, title_zh, title_en, circle_name, description
        );
        INSERT INTO works_fts (rowid, rj_code, title_ja, title_zh, title_en, circle_name, description)
            SELECT id, rj_code, title_ja, title_zh, title_en, circle_name, description FROM works;

        -- ==================== 旧库迁移：local_files 唯一约束 ====================
        -- 清理可能的历史重复行（保留每个 work_id+local_path 的最小 id）
        DELETE FROM local_files
        WHERE id NOT IN (
            SELECT MIN(id) FROM local_files GROUP BY work_id, local_path
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_local_files_work_path
        ON local_files(work_id, local_path);
        "#,
    )?;
    Ok(())
}
