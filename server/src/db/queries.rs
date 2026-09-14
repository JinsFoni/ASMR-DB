use super::models::*;
use rusqlite::{params, Connection, OptionalExtension, Row};

// ============================= Works =============================

pub fn row_to_work(row: &Row) -> rusqlite::Result<Work> {
    Ok(Work {
        id: row.get("id")?,
        rj_code: row.get("rj_code")?,
        title_ja: row.get("title_ja")?,
        title_zh: row.get("title_zh")?,
        title_en: row.get("title_en")?,
        circle_name: row.get("circle_name")?,
        circle_id: row.get("circle_id")?,
        cover_url: row.get("cover_url")?,
        work_type: row.get("work_type")?,
        price: row.get("price")?,
        sale_date: row.get("sale_date")?,
        description: row.get("description")?,
        age_class: row.get("age_class")?,
        duration_min: row.get("duration_min")?,
        file_size_mb: row.get("file_size_mb")?,
        dlsite_url: row.get("dlsite_url")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// 列出作品（支持搜索、筛选、排序、分页）。返回 (works_with_meta, total)
pub fn list_works(
    conn: &Connection,
    search: Option<&str>,
    status_filter: Option<&str>,
    tag_filter: Option<i64>,
    group_filter: Option<i64>,
    sort_by: &str,
    page: u32,
    per_page: u32,
) -> Result<(Vec<WorkView>, u32), rusqlite::Error> {
    // Build base SQL — use LIKE for CJK-friendly substring search (library sizes are small).
    let base_sql = if let Some(kw) = search {
        let kw = kw.trim();
        if !kw.is_empty() {
            let pat = format!("%{}%", kw.replace(['%', '_', '\''], ""));
            format!(
                "FROM works w
                 WHERE (w.rj_code LIKE '{}' COLLATE NOCASE
                     OR w.title_ja LIKE '{}'
                     OR w.title_zh LIKE '{}'
                     OR w.title_en LIKE '{}'
                     OR w.circle_name LIKE '{}')",
                pat, pat, pat, pat, pat
            )
        } else {
            "FROM works w".to_string()
        }
    } else {
        "FROM works w".to_string()
    };

    let mut sql = base_sql.clone();
    let mut has_where = sql.contains(" WHERE ");
    if let Some(st) = status_filter {
        let cond = match st {
            "downloaded" => {
                " EXISTS (SELECT 1 FROM local_files lf WHERE lf.work_id = w.id AND lf.download_status = 'downloaded')"
            }
            "not_downloaded" => {
                " NOT EXISTS (SELECT 1 FROM local_files lf WHERE lf.work_id = w.id AND lf.download_status = 'downloaded')"
            }
            "downloading" => {
                " EXISTS (SELECT 1 FROM local_files lf WHERE lf.work_id = w.id AND lf.download_status = 'downloading')"
            }
            _ => "",
        };
        if !cond.is_empty() {
            if has_where {
                sql.push_str(" AND");
            } else {
                sql.push_str(" WHERE");
                has_where = true;
            }
            sql.push_str(cond);
        }
    }
    if let Some(tid) = tag_filter {
        let cond = format!(
            " EXISTS (SELECT 1 FROM work_tags wt WHERE wt.work_id = w.id AND wt.tag_id = {})",
            tid
        );
        if has_where {
            sql.push_str(" AND");
        } else {
            sql.push_str(" WHERE");
            has_where = true;
        }
        sql.push_str(&cond);
    }
    if let Some(gid) = group_filter {
        let cond = format!(
            " EXISTS (SELECT 1 FROM work_groups wg WHERE wg.work_id = w.id AND wg.group_id = {})",
            gid
        );
        if has_where {
            sql.push_str(" AND");
        } else {
            sql.push_str(" WHERE");
            has_where = true;
        }
        sql.push_str(&cond);
    }

    let sort_col = match sort_by {
        "title" => "w.title_ja COLLATE NOCASE",
        "circle" => "w.circle_name COLLATE NOCASE",
        "sale_date" => "w.sale_date",
        _ => "w.id",
    };
    let sort_dir = if sort_by == "sale_date" { "DESC" } else { "DESC" };
    sql.push_str(&format!(" ORDER BY {} {}", sort_col, sort_dir));

    // Count
    let count_sql = format!("SELECT COUNT(*) {};", sql);
    let total: u32 = conn.query_row(&count_sql, [], |r| r.get(0))?;

    // Paginate
    let offset = (page.saturating_sub(1)) * per_page;
    let page_sql = format!(
        "SELECT w.* {} LIMIT {} OFFSET {};",
        sql, per_page, offset
    );
    let mut stmt = conn.prepare(&page_sql)?;
    let rows = stmt.query_map([], |row| {
        let work = row_to_work(row)?;
        Ok(work)
    })?;

    let mut works: Vec<Work> = Vec::new();
    for r in rows {
        works.push(r?);
    }

    let mut views = Vec::new();
    for w in works {
        views.push(work_view(conn, w)?);
    }

    Ok((views, total))
}

/// 构建单条作品的完整视图（含标签、声优、下载状态、音轨信息、分组）
pub fn work_view(conn: &Connection, work: Work) -> rusqlite::Result<WorkView> {
    let tags = list_work_tags(conn, work.id)?;
    let actors = list_work_actors(conn, work.id)?;
    let (status, local_path) = work_download_status(conn, work.id)?;
    let track_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audio_tracks WHERE work_id = ?1",
        params![work.id],
        |r| r.get(0),
    )?;
    let total_duration: f64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_sec),0) FROM audio_tracks WHERE work_id = ?1",
        params![work.id],
        |r| r.get(0),
    )?;
    let group = get_work_group(conn, work.id)?;
    Ok(WorkView {
        work,
        tags,
        actors,
        download_status: status,
        local_path,
        track_count,
        total_duration_sec: total_duration,
        group,
    })
}

pub fn get_work(conn: &Connection, id: i64) -> Result<Option<WorkView>, rusqlite::Error> {
    let work: Option<Work> = conn
        .query_row("SELECT * FROM works WHERE id = ?1", params![id], row_to_work)
        .optional()?;
    match work {
        Some(w) => Ok(Some(work_view(conn, w)?)),
        None => Ok(None),
    }
}

pub fn get_work_by_rj(conn: &Connection, rj: &str) -> Result<Option<Work>, rusqlite::Error> {
    conn.query_row(
        "SELECT * FROM works WHERE rj_code = ?1 COLLATE NOCASE",
        params![rj],
        row_to_work,
    )
    .optional()
}

/// 插入作品；若 RJ 号已存在则更新。返回 work id。
pub fn upsert_work(conn: &Connection, w: &ScrapedWork) -> Result<i64, rusqlite::Error> {
    conn.execute(
        r#"INSERT INTO works (rj_code, title_ja, title_zh, title_en, circle_name, circle_id,
            cover_url, work_type, price, sale_date, description, age_class, duration_min, file_size_mb, dlsite_url)
           VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
           ON CONFLICT(rj_code) DO UPDATE SET
            title_ja=excluded.title_ja, title_zh=excluded.title_zh, title_en=excluded.title_en,
            circle_name=excluded.circle_name, circle_id=excluded.circle_id, cover_url=excluded.cover_url,
            work_type=excluded.work_type, price=excluded.price, sale_date=excluded.sale_date,
            description=excluded.description, age_class=excluded.age_class,
            duration_min=excluded.duration_min, file_size_mb=excluded.file_size_mb,
            dlsite_url=excluded.dlsite_url, updated_at=datetime('now')"#,
        params![
            w.rj_code,
            w.title_ja,
            w.title_zh,
            w.title_en,
            w.circle_name,
            w.circle_id,
            w.cover_url,
            w.work_type,
            w.price,
            w.sale_date,
            w.description,
            w.age_class,
            w.duration_min,
            w.file_size_mb,
            w.dlsite_url,
        ],
    )?;

    let id = get_work_by_rj(conn, &w.rj_code)?.unwrap().id;

    // FTS5 虚拟表不支持 UPSERT（ON CONFLICT DO UPDATE），需先删后插
    conn.execute("DELETE FROM works_fts WHERE rowid = ?1", params![id])?;
    conn.execute(
        r#"INSERT INTO works_fts (rowid, rj_code, title_ja, title_zh, title_en, circle_name, description)
           VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
        params![
            id,
            w.rj_code,
            w.title_ja,
            w.title_zh,
            w.title_en,
            w.circle_name,
            w.description,
        ],
    )?;

    Ok(id)
}

pub fn update_work(conn: &Connection, w: &Work) -> Result<(), rusqlite::Error> {
    conn.execute(
        r#"UPDATE works SET title_ja=?1, title_zh=?2, title_en=?3, circle_name=?4,
            cover_url=?5, work_type=?6, price=?7, sale_date=?8, description=?9,
            age_class=?10, duration_min=?11, file_size_mb=?12, dlsite_url=?13,
            updated_at=datetime('now') WHERE id=?14"#,
        params![
            w.title_ja,
            w.title_zh,
            w.title_en,
            w.circle_name,
            w.cover_url,
            w.work_type,
            w.price,
            w.sale_date,
            w.description,
            w.age_class,
            w.duration_min,
            w.file_size_mb,
            w.dlsite_url,
            w.id,
        ],
    )?;
    Ok(())
}

pub fn delete_work(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM works_fts WHERE rowid = ?1", params![id])?;
    conn.execute("DELETE FROM works WHERE id = ?1", params![id])?;
    Ok(())
}

/// 批量删除作品（仅删除数据库记录，不动本地文件）。
pub fn delete_works(conn: &Connection, ids: &[i64]) -> Result<(), rusqlite::Error> {
    if ids.is_empty() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare("DELETE FROM works_fts WHERE rowid = ?1")?;
        for id in ids {
            stmt.execute(params![id])?;
        }
    }
    // 批量删除 works，CASCADE 自动清理 work_groups / audio_tracks / local_files / play_history 等
    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!("DELETE FROM works WHERE id IN ({})", placeholders.join(","));
    {
        let mut stmt = tx.prepare(&sql)?;
        stmt.execute(rusqlite::params_from_iter(ids.iter()))?;
    }
    tx.commit()?;
    Ok(())
}

// ============================= Groups =============================

/// 获取作品所属分组。
pub fn get_work_group(conn: &Connection, work_id: i64) -> Result<Option<Group>, rusqlite::Error> {
    conn.query_row(
        "SELECT g.id, g.name, g.color FROM work_groups wg
         JOIN groups g ON g.id = wg.group_id
         WHERE wg.work_id = ?1",
        params![work_id],
        |row| {
            Ok(Group {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        },
    )
    .optional()
}

/// 列出所有分组。
pub fn list_groups(conn: &Connection) -> Result<Vec<Group>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, name, color FROM groups ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Group {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// 创建分组（已存在则直接返回其 id）。
pub fn create_group(conn: &Connection, name: &str, color: &str) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO groups (name, color) VALUES (?1, ?2)",
        params![name, color],
    )?;
    conn.query_row("SELECT id FROM groups WHERE name = ?1", params![name], |r| {
        r.get(0)
    })
}

/// 设置作品分组；group_id 为 None 表示取消分组。
pub fn set_work_group(
    conn: &Connection,
    work_id: i64,
    group_id: Option<i64>,
) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM work_groups WHERE work_id = ?1", params![work_id])?;
    if let Some(gid) = group_id {
        conn.execute(
            "INSERT INTO work_groups (work_id, group_id) VALUES (?1, ?2)",
            params![work_id, gid],
        )?;
    }
    Ok(())
}

/// 删除分组（同时清理 work_groups 关联）。
pub fn delete_group(conn: &Connection, group_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM work_groups WHERE group_id = ?1", params![group_id])?;
    conn.execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
    Ok(())
}

// ============================= Tags =============================

pub fn list_tags(conn: &Connection) -> Result<Vec<Tag>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT * FROM tags ORDER BY is_preset DESC, name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get("id")?,
            name: row.get("name")?,
            color: row.get("color")?,
            is_preset: row.get::<_, i64>("is_preset")? != 0,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn create_tag(conn: &Connection, name: &str, color: &str) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO tags (name, color) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET name=excluded.name",
        params![name, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn assign_tag(conn: &Connection, work_id: i64, tag_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO work_tags (work_id, tag_id, is_auto) VALUES (?1, ?2, 0)",
        params![work_id, tag_id],
    )?;
    Ok(())
}

pub fn unassign_tag(conn: &Connection, work_id: i64, tag_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM work_tags WHERE work_id = ?1 AND tag_id = ?2",
        params![work_id, tag_id],
    )?;
    Ok(())
}

pub fn delete_tag(conn: &Connection, tag_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM tags WHERE id = ?1", params![tag_id])?;
    Ok(())
}

fn list_work_tags(conn: &Connection, work_id: i64) -> Result<Vec<Tag>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT t.* FROM tags t JOIN work_tags wt ON wt.tag_id = t.id WHERE wt.work_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(params![work_id], |row| {
        Ok(Tag {
            id: row.get("id")?,
            name: row.get("name")?,
            color: row.get("color")?,
            is_preset: row.get::<_, i64>("is_preset")? != 0,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// ============================= Actors =============================

fn list_work_actors(conn: &Connection, work_id: i64) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT va.name FROM voice_actors va JOIN work_actors wa ON wa.actor_id = va.id WHERE wa.work_id = ?1",
    )?;
    let rows = stmt.query_map(params![work_id], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn list_all_actors(conn: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT DISTINCT name FROM voice_actors ORDER BY name")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// 为作品设置声优列表（先清空再插入）
pub fn set_work_actors(conn: &Connection, work_id: i64, names: &[String]) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM work_actors WHERE work_id = ?1", params![work_id])?;
    for name in names {
        if name.trim().is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO voice_actors (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![name],
        )?;
        let actor_id: i64 = conn.query_row(
            "SELECT id FROM voice_actors WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO work_actors (work_id, actor_id) VALUES (?1, ?2)",
            params![work_id, actor_id],
        )?;
    }
    Ok(())
}

/// 为作品设置标签（仅自动标签，非覆盖用户标签）
pub fn set_work_auto_tags(conn: &Connection, work_id: i64, names: &[String]) -> Result<(), rusqlite::Error> {
    for name in names {
        if name.trim().is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO tags (name, color, is_preset) VALUES (?1, '#888888', 1)
             ON CONFLICT(name) DO NOTHING",
            params![name],
        )?;
        let tag_id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        // Only add if not manually assigned, to avoid overriding manual color
        conn.execute(
            "INSERT OR IGNORE INTO work_tags (work_id, tag_id, is_auto) VALUES (?1, ?2, 1)",
            params![work_id, tag_id],
        )?;
    }
    Ok(())
}

// ============================= Tracks =============================

pub fn list_tracks(conn: &Connection, work_id: i64) -> Result<Vec<Track>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT * FROM audio_tracks WHERE work_id = ?1 ORDER BY track_number, file_path",
    )?;
    let rows = stmt.query_map(params![work_id], |row| {
        Ok(Track {
            id: row.get("id")?,
            work_id: row.get("work_id")?,
            file_path: row.get("file_path")?,
            track_number: row.get("track_number")?,
            title: row.get("title")?,
            duration_sec: row.get("duration_sec")?,
            file_format: row.get("file_format")?,
            file_size: row.get("file_size")?,
            subtitles: Vec::new(),
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn clear_tracks(conn: &Connection, work_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM audio_tracks WHERE work_id = ?1", params![work_id])?;
    Ok(())
}

pub fn insert_track(conn: &Connection, t: &Track) -> Result<(), rusqlite::Error> {
    conn.execute(
        r#"INSERT INTO audio_tracks (work_id, file_path, track_number, title, duration_sec, file_format, file_size)
           VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
        params![
            t.work_id,
            t.file_path,
            t.track_number,
            t.title,
            t.duration_sec,
            t.file_format,
            t.file_size,
        ],
    )?;
    Ok(())
}

// ============================= Local files =============================

pub fn upsert_local_file(conn: &Connection, work_id: i64, path: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        r#"INSERT INTO local_files (work_id, local_path, file_type, download_status)
           VALUES (?1, ?2, 'dir', 'downloaded')
           ON CONFLICT(work_id, local_path) DO UPDATE SET download_status='downloaded'"#,
        params![work_id, path],
    )?;
    Ok(())
}

pub fn work_download_status(conn: &Connection, work_id: i64) -> Result<(String, Option<String>), rusqlite::Error> {
    let result: Option<(String, String)> = conn
        .query_row(
            "SELECT download_status, local_path FROM local_files WHERE work_id = ?1 ORDER BY download_status = 'downloaded' DESC, id DESC LIMIT 1",
            params![work_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(match result {
        Some((s, p)) => (s, Some(p)),
        None => ("not_downloaded".to_string(), None),
    })
}

// ============================= Play history =============================

pub fn save_play_progress(
    conn: &Connection,
    work_id: i64,
    track_id: Option<i64>,
    position: f64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        r#"INSERT INTO play_history (work_id, track_id, last_position, play_count, last_played_at)
           VALUES (?1, ?2, ?3, 1, datetime('now'))
           ON CONFLICT(work_id, track_id) DO UPDATE SET
             last_position=excluded.last_position,
             play_count=play_count+1,
             last_played_at=datetime('now')"#,
        params![work_id, track_id, position],
    )?;
    Ok(())
}

pub fn get_play_progress(conn: &Connection, work_id: i64) -> Result<Option<PlayHistory>, rusqlite::Error> {
    conn.query_row(
        "SELECT * FROM play_history WHERE work_id = ?1 ORDER BY last_played_at DESC LIMIT 1",
        params![work_id],
        |row| {
            Ok(PlayHistory {
                id: row.get("id")?,
                work_id: row.get("work_id")?,
                track_id: row.get("track_id")?,
                last_position: row.get("last_position")?,
                play_count: row.get("play_count")?,
                last_played_at: row.get("last_played_at")?,
            })
        },
    )
    .optional()
}

// ============================= Settings =============================

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// 播放历史对应的作品（按最近播放时间倒序，去重到作品级）。返回 (作品, 是否还有更多)。
pub fn list_play_history_works(
    conn: &Connection,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Work>, bool), rusqlite::Error> {
    let mut stmt = conn.prepare(
        r#"SELECT w.* FROM works w
           JOIN (SELECT work_id, MAX(last_played_at) AS m FROM play_history GROUP BY work_id) ph
             ON ph.work_id = w.id
           ORDER BY ph.m DESC
           LIMIT ?1 OFFSET ?2"#,
    )?;
    let rows = stmt
        .query_map(params![per_page + 1, (page - 1) * per_page], row_to_work)?
        .collect::<rusqlite::Result<Vec<Work>>>()?;
    let has_more = rows.len() as i64 > per_page;
    let mut rows = rows;
    if has_more {
        rows.truncate(per_page as usize);
    }
    Ok((rows, has_more))
}

// ============================= DLsite 排行榜 =============================

/// 用新抓取的数据全量替换某个榜单周期的条目（fetched_at 重置为当前时间）。
/// position 为抓取顺序号（1..N，主键）；rank 为 DLsite 展示名次，允许并列。
pub fn replace_rankings(
    conn: &Connection,
    term: &str,
    entries: &[RankingEntry],
) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM dlsite_rankings WHERE term = ?1", params![term])?;
    let mut stmt = conn.prepare(
        "INSERT INTO dlsite_rankings
            (term, position, rank, rj_code, title, circle_name, cover_url, price, sale_date, dl_count, rating, tags_json)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
    )?;
    for (idx, e) in entries.iter().enumerate() {
        let tags_json = serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".into());
        stmt.execute(params![
            term,
            (idx + 1) as i64,
            e.rank,
            e.rj_code,
            e.title,
            e.circle_name,
            e.cover_url,
            e.price,
            e.sale_date,
            e.dl_count,
            e.rating,
            tags_json,
        ])?;
    }
    Ok(())
}

/// 读取榜单前 limit 条，LEFT JOIN works 带出本地入库状态（work_id / download_status）。
pub fn list_rankings(
    conn: &Connection,
    term: &str,
    limit: i64,
) -> Result<Vec<DlsiteRankingItem>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        r#"SELECT r.rank, r.rj_code, r.title, r.circle_name, r.cover_url, r.price, r.sale_date,
                  r.dl_count, r.rating, r.tags_json, r.fetched_at, w.id,
                  (SELECT MAX(CASE WHEN lf.download_status = 'downloaded' THEN 2
                                   WHEN lf.download_status = 'downloading' THEN 1
                                   ELSE 0 END)
                   FROM local_files lf WHERE lf.work_id = w.id) AS st
           FROM dlsite_rankings r
           LEFT JOIN works w ON w.rj_code = r.rj_code
           WHERE r.term = ?1
           ORDER BY r.position
           LIMIT ?2"#,
    )?;
    let rows = stmt.query_map(params![term, limit], |row| {
        let tags_json: Option<String> = row.get(9)?;
        let work_id: Option<i64> = row.get(11)?;
        let st: Option<i64> = row.get(12)?;
        let download_status = work_id.map(|_| match st.unwrap_or(0) {
            2 => "downloaded",
            1 => "downloading",
            _ => "not_downloaded",
        });
        Ok(DlsiteRankingItem {
            rank: row.get(0)?,
            rj_code: row.get(1)?,
            title: row.get(2)?,
            circle_name: row.get(3)?,
            cover_url: row.get(4)?,
            price: row.get(5)?,
            sale_date: row.get(6)?,
            dl_count: row.get(7)?,
            rating: row.get(8)?,
            tags: tags_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            work_id,
            download_status: download_status.map(String::from),
            fetched_at: row.get(10)?,
        })
    })?;
    rows.collect()
}

/// 榜单最近一次抓取时间（SQLite UTC 时间字符串）。
pub fn ranking_last_fetch(
    conn: &Connection,
    term: &str,
) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT MAX(fetched_at) FROM dlsite_rankings WHERE term = ?1",
        params![term],
        |r| r.get(0),
    )
}

/// 判断是否需要刷新：任一周期没有数据或数据早于 max_age_hours 小时前 → true。
pub fn rankings_stale(
    conn: &Connection,
    terms: &[&str],
    max_age_hours: i64,
) -> Result<bool, rusqlite::Error> {
    for term in terms {
        let fresh: i64 = conn.query_row(
            "SELECT COUNT(*) FROM dlsite_rankings
             WHERE term = ?1 AND fetched_at >= datetime('now', ?2)",
            params![term, format!("-{max_age_hours} hours")],
            |r| r.get(0),
        )?;
        if fresh == 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建内存数据库并建好 list_works / work_view 所需的完整 schema。
    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE works (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                rj_code TEXT NOT NULL UNIQUE,
                title_ja TEXT, title_zh TEXT, title_en TEXT,
                circle_name TEXT, circle_id TEXT, cover_url TEXT,
                work_type TEXT NOT NULL DEFAULT 'voice',
                price INTEGER, sale_date TEXT, description TEXT,
                age_class TEXT, duration_min INTEGER, file_size_mb REAL,
                dlsite_url TEXT, created_at TEXT, updated_at TEXT
            );
            CREATE TABLE tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, color TEXT DEFAULT '#888888', is_preset INTEGER DEFAULT 0);
            CREATE TABLE work_tags (work_id INTEGER NOT NULL, tag_id INTEGER NOT NULL, is_auto INTEGER DEFAULT 0,
                PRIMARY KEY (work_id, tag_id),
                FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE);
            CREATE TABLE voice_actors (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, dlsite_url TEXT);
            CREATE TABLE work_actors (work_id INTEGER NOT NULL, actor_id INTEGER NOT NULL,
                PRIMARY KEY (work_id, actor_id),
                FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
                FOREIGN KEY (actor_id) REFERENCES voice_actors(id) ON DELETE CASCADE);
            CREATE TABLE local_files (id INTEGER PRIMARY KEY AUTOINCREMENT, work_id INTEGER NOT NULL, local_path TEXT NOT NULL,
                file_type TEXT DEFAULT 'archive', download_status TEXT DEFAULT 'not_downloaded',
                download_progress REAL DEFAULT 0, file_size INTEGER, created_at TEXT,
                FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
                UNIQUE (work_id, local_path));
            CREATE TABLE audio_tracks (id INTEGER PRIMARY KEY AUTOINCREMENT, work_id INTEGER NOT NULL, file_path TEXT NOT NULL,
                track_number INTEGER, title TEXT, duration_sec REAL, file_format TEXT, file_size INTEGER,
                FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE);
            CREATE TABLE groups (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, color TEXT DEFAULT '#597ef7');
            CREATE TABLE work_groups (work_id INTEGER PRIMARY KEY, group_id INTEGER NOT NULL,
                FOREIGN KEY (work_id) REFERENCES works(id) ON DELETE CASCADE,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE);
            CREATE TABLE dlsite_rankings (
                term TEXT NOT NULL, position INTEGER NOT NULL, rank INTEGER NOT NULL,
                rj_code TEXT NOT NULL, title TEXT, circle_name TEXT, cover_url TEXT,
                price INTEGER, sale_date TEXT, dl_count INTEGER, rating REAL, tags_json TEXT,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (term, position)
            );
            CREATE VIRTUAL TABLE works_fts USING fts5(rj_code, title_ja, title_zh, title_en, circle_name, description);
            "#,
        )
        .unwrap();
        conn
    }

    fn insert_work(conn: &Connection, rj: &str) -> i64 {
        conn.execute(
            "INSERT INTO works (rj_code, title_ja) VALUES (?1, ?2)",
            params![rj, rj],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_create_group_reuses_existing() {
        let conn = test_conn();
        let g1 = create_group(&conn, "耳かき", "#597ef7").unwrap();
        let g2 = create_group(&conn, "耳かき", "#597ef7").unwrap();
        assert_eq!(g1, g2);
        assert_eq!(list_groups(&conn).unwrap().len(), 1);
    }

    #[test]
    fn test_set_and_get_work_group() {
        let conn = test_conn();
        let wid = insert_work(&conn, "RJ000001");
        let gid = create_group(&conn, "添い寝", "#597ef7").unwrap();
        set_work_group(&conn, wid, Some(gid)).unwrap();
        let g = get_work_group(&conn, wid).unwrap().unwrap();
        assert_eq!(g.name, "添い寝");
        // 一个作品只属于一个分组：再次设置会替换
        let gid2 = create_group(&conn, "耳かき", "#597ef7").unwrap();
        set_work_group(&conn, wid, Some(gid2)).unwrap();
        let g2 = get_work_group(&conn, wid).unwrap().unwrap();
        assert_eq!(g2.id, gid2);
        assert_eq!(list_groups(&conn).unwrap().len(), 2);
        // 取消分组
        set_work_group(&conn, wid, None).unwrap();
        assert!(get_work_group(&conn, wid).unwrap().is_none());
    }

    #[test]
    fn test_delete_works_batch_and_cascade() {
        let conn = test_conn();
        let w1 = insert_work(&conn, "RJ000001");
        let w2 = insert_work(&conn, "RJ000002");
        insert_work(&conn, "RJ000003");
        let gid = create_group(&conn, "耳かき", "#597ef7").unwrap();
        set_work_group(&conn, w1, Some(gid)).unwrap();
        set_work_group(&conn, w2, Some(gid)).unwrap();

        delete_works(&conn, &[w1, w2]).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM works", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let wg: i64 = conn
            .query_row("SELECT COUNT(*) FROM work_groups", [], |r| r.get(0))
            .unwrap();
        assert_eq!(wg, 0, "work_groups 应被级联清理");
        assert!(get_work_group(&conn, w1).unwrap().is_none());

        // 空列表调用应安全返回
        delete_works(&conn, &[]).unwrap();
    }

    #[test]
    fn test_list_works_group_filter() {
        let conn = test_conn();
        let w1 = insert_work(&conn, "RJ000001");
        let w2 = insert_work(&conn, "RJ000002");
        let gid = create_group(&conn, "耳かき", "#597ef7").unwrap();
        set_work_group(&conn, w1, Some(gid)).unwrap();

        let (views, total) = list_works(&conn, None, None, None, Some(gid), "id", 1, 50).unwrap();
        assert_eq!(total, 1);
        assert_eq!(views[0].work.id, w1);
        assert!(views[0].group.is_some());
        assert_eq!(views[0].group.as_ref().unwrap().name, "耳かき");

        let (all, total_all) = list_works(&conn, None, None, None, None, "id", 1, 50).unwrap();
        assert_eq!(total_all, 2);
        // w2 无分组
        let w2_view = all.iter().find(|v| v.work.id == w2).unwrap();
        assert!(w2_view.group.is_none());
    }

    fn ranking_entry(rank: i64, rj: &str) -> RankingEntry {
        RankingEntry {
            rank,
            rj_code: rj.into(),
            title: Some("测试作品".into()),
            circle_name: Some("测试社团".into()),
            cover_url: None,
            price: Some(1320),
            sale_date: Some("2026-01-01".into()),
            dl_count: Some(100),
            rating: Some(4.5),
            tags: vec!["耳かき".into()],
        }
    }

    #[test]
    fn test_rankings_replace_and_library_join() {
        let conn = test_conn();
        let wid = insert_work(&conn, "RJ000001");
        conn.execute(
            "INSERT INTO local_files (work_id, local_path, download_status) VALUES (?1, '/tmp/x', 'downloaded')",
            params![wid],
        )
        .unwrap();

        // RJ000001 已入库且已下载；RJ000002 / RJ000003 未入库，RJ000002 与 RJ000003 并列第 2
        let entries = vec![
            ranking_entry(1, "RJ000001"),
            ranking_entry(2, "RJ000002"),
            ranking_entry(2, "RJ000003"),
        ];
        replace_rankings(&conn, "day", &entries).unwrap();

        let items = list_rankings(&conn, "day", 20).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].work_id, Some(wid));
        assert_eq!(items[0].download_status.as_deref(), Some("downloaded"));
        assert_eq!(items[1].work_id, None);
        assert_eq!(items[1].download_status, None);
        assert_eq!(items[2].rank, 2, "并列名次应保留");
        assert_eq!(items[0].tags, vec!["耳かき".to_string()]);

        // limit 生效 + 全量替换不残留旧行
        assert_eq!(list_rankings(&conn, "day", 2).unwrap().len(), 2);
        replace_rankings(&conn, "day", &entries[..1]).unwrap();
        assert_eq!(list_rankings(&conn, "day", 20).unwrap().len(), 1);

        // 过期检查：刚写入的 day 不需要刷新，week 无数据需要刷新
        assert!(!rankings_stale(&conn, &["day"], 4).unwrap());
        assert!(rankings_stale(&conn, &["day", "week"], 4).unwrap());
    }
}
