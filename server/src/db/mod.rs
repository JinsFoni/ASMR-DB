pub mod models;
pub mod queries;

use std::path::Path;

use rusqlite::Connection;

/// 打开（或创建）SQLite 数据库并执行迁移。
pub fn init_database(db_path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(db_path)?;
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
            description_zh  TEXT,
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

        -- ==================== DLsite 排行榜快照表 ====================
        -- 每个榜单周期（day/week/month/year/total）各存 Top 100，
        -- 定时任务每 4 小时全量替换，fetched_at 为该行抓取时间。
        -- position 为榜单内序号（主键，保证唯一）；rank 为 DLsite 展示名次，
        -- DLsite 榜单存在并列名次（如三作品并列第 76），不可作唯一键。
        CREATE TABLE IF NOT EXISTS dlsite_rankings (
            term        TEXT    NOT NULL,
            position    INTEGER NOT NULL,
            rank        INTEGER NOT NULL,
            rj_code     TEXT    NOT NULL,
            title       TEXT,
            circle_name TEXT,
            cover_url   TEXT,
            price       INTEGER,
            sale_date   TEXT,
            dl_count    INTEGER,
            rating      REAL,
            tags_json   TEXT,
            fetched_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (term, position)
        );

        -- ==================== 索引 ====================
        CREATE INDEX IF NOT EXISTS idx_works_rj ON works(rj_code);
        CREATE INDEX IF NOT EXISTS idx_works_circle ON works(circle_name);
        CREATE INDEX IF NOT EXISTS idx_works_type ON works(work_type);
        CREATE INDEX IF NOT EXISTS idx_audio_work ON audio_tracks(work_id);
        CREATE INDEX IF NOT EXISTS idx_dlsite_rankings_rj ON dlsite_rankings(rj_code);
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

        -- ==================== play_history 唯一约束 ====================
        -- save_play_progress 使用 ON CONFLICT(work_id, track_id)，需要唯一索引支撑。
        -- track_id 为 NULL 时唯一索引不冲突（SQLite 视 NULL 为互异），符合预期。
        DELETE FROM play_history
        WHERE id NOT IN (
            SELECT MIN(id) FROM play_history GROUP BY work_id, track_id
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_play_history_work_track
        ON play_history(work_id, track_id);

        -- ==================== LLM 翻译任务表 ====================
        -- 每部作品一个任务（work_id 唯一，重试/重译复用同一行）。
        CREATE TABLE IF NOT EXISTS translation_tasks (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            work_id         INTEGER NOT NULL UNIQUE,
            status          TEXT    NOT NULL DEFAULT 'pending',
            source_title    TEXT,
            translated_title TEXT,
            source_desc     TEXT,
            translated_desc TEXT,
            error           TEXT,
            retry_count     INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT    DEFAULT (datetime('now')),
            updated_at      TEXT    DEFAULT (datetime('now')),
            FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_translation_tasks_status
        ON translation_tasks(status);
        "#,
    )?;

    // 旧库补列：works.description_zh（LLM 翻译的中文简介，已存在则忽略）
    let _ = conn.execute("ALTER TABLE works ADD COLUMN description_zh TEXT", []);

    Ok(())
}
