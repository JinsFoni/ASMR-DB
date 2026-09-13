use crate::db::models::*;
use crate::db::queries;
use crate::AppState;
use serde::Deserialize;
use tauri::State;

type Db<'a> = State<'a, AppState>;

// ============================= Works =============================

/// Tauri 嵌套结构不会自动把 camelCase 转成 snake_case，
/// 所以这里必须显式声明，否则 JS 端的 tagId/groupId/sortBy/perPage 全部无法解析。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkListQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub tag_id: Option<i64>,
    pub group_id: Option<i64>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[tauri::command]
pub fn list_works(db: Db, q: WorkListQuery) -> Result<serde_json::Value, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let (items, total) = queries::list_works(
        &conn,
        q.search.as_deref(),
        q.status.as_deref(),
        q.tag_id,
        q.group_id,
        q.sort_by.as_deref().unwrap_or("id"),
        q.page.unwrap_or(1),
        q.per_page.unwrap_or(48),
    )
    .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "items": items, "total": total }))
}

#[tauri::command]
pub fn get_work(db: Db, id: i64) -> Result<Option<WorkView>, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::get_work(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_work(db: Db, work: Work) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::update_work(&conn, &work).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_work(db: Db, id: i64) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::delete_work(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_works(db: Db, ids: Vec<i64>) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::delete_works(&conn, &ids).map_err(|e| e.to_string())
}

// ============================= Groups =============================

#[tauri::command]
pub fn list_groups(db: Db) -> Result<Vec<crate::db::models::Group>, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::list_groups(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_group(db: Db, name: String, color: Option<String>) -> Result<i64, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::create_group(&conn, &name, &color.unwrap_or_else(|| "#597ef7".into()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_work_group(db: Db, work_id: i64, group_id: Option<i64>) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::set_work_group(&conn, work_id, group_id).map_err(|e| e.to_string())
}

/// 删除分组（同时清理作品与分组的关联）。
#[tauri::command]
pub fn delete_group(db: Db, group_id: i64) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::delete_group(&conn, group_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_download_status(db: Db, id: i64) -> Result<String, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let (status, _) = queries::work_download_status(&conn, id).map_err(|e| e.to_string())?;
    Ok(status)
}

// ============================= Tags =============================

#[tauri::command]
pub fn list_tags(db: Db) -> Result<Vec<Tag>, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::list_tags(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_tag(db: Db, name: String, color: String) -> Result<i64, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::create_tag(&conn, &name, &color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn assign_tag(db: Db, work_id: i64, tag_id: i64) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::assign_tag(&conn, work_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unassign_tag(db: Db, work_id: i64, tag_id: i64) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::unassign_tag(&conn, work_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tag(db: Db, tag_id: i64) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::delete_tag(&conn, tag_id).map_err(|e| e.to_string())
}

// ============================= Actors =============================

#[tauri::command]
pub fn list_actors(db: Db) -> Result<Vec<String>, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::list_all_actors(&conn).map_err(|e| e.to_string())
}

// ============================= Tracks =============================

#[tauri::command]
pub fn list_tracks(db: Db, work_id: i64) -> Result<Vec<Track>, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::list_tracks(&conn, work_id).map_err(|e| e.to_string())
}

// ============================= Play history =============================

#[tauri::command]
pub fn save_play_progress(db: Db, work_id: i64, track_id: Option<i64>, position: f64) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::save_play_progress(&conn, work_id, track_id, position).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_play_progress(db: Db, work_id: i64) -> Result<Option<PlayHistory>, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::get_play_progress(&conn, work_id).map_err(|e| e.to_string())
}

// ============================= asmr.one 在线试听 =============================

/// 拉取 asmr.one 上某作品的音频文件列表，构造为临时 `Track` 返回给前端在线播放。
/// `file_path` 直接用 asmr.one 公开 CDN 的 `mediaStreamUrl`（raw.kiko-play-niptan.one，
/// 无需 token，CDN 支持 Range seek），前端 Howler 直接加载。不写 DB。
#[tauri::command]
pub fn list_asmrone_tracks(db: Db, rj_code: String) -> Result<Vec<Track>, String> {
    let token = {
        let conn = db.db.lock().map_err(|e| e.to_string())?;
        queries::get_setting(&conn, "asmr_one_token")
            .ok()
            .flatten()
            .unwrap_or_default()
    };
    let files = crate::api::asmrone::fetch_audio_files(&rj_code, Some(&token))?;
    if files.is_empty() {
        return Err("asmr.one 未返回音频文件，可能需要登录或该作品无音频".to_string());
    }
    let tracks = files
        .into_iter()
        .enumerate()
        .filter_map(|(idx, f)| {
            // 只返回带可播放 URL 的（个别节点可能缺 URL，直接跳过）
            if f.stream_url.trim().is_empty() {
                return None;
            }
            Some(Track {
                id: idx as i64,
                work_id: 0,
                file_path: f.stream_url,
                track_number: Some((idx + 1) as i64),
                title: Some(f.title),
                duration_sec: f.duration_sec,
                file_format: Some(f.media_extension),
                file_size: f.size_bytes,
                subtitles: f.subtitles,
            })
        })
        .collect();
    Ok(tracks)
}

/// 获取 asmr.one 上某作品的完整文件树（含所有类型），用于下载文件选择界面。
#[tauri::command]
pub fn list_asmrone_tree(
    db: Db,
    rj_code: String,
) -> Result<Vec<crate::api::asmrone::AsmrTreeNode>, String> {
    let token = {
        let conn = db.db.lock().map_err(|e| e.to_string())?;
        queries::get_setting(&conn, "asmr_one_token")
            .unwrap_or(None)
            .unwrap_or_default()
    };
    crate::api::asmrone::fetch_file_tree(&rj_code, Some(&token))
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    #[test]
    fn test_work_list_query_camel_case() {
        // 模拟 JS 发送的 camelCase 参数（Tauri 嵌套结构不自动转换）
        let q: WorkListQuery = serde_json::from_str(r#"{"search":"foo","groupId":3,"tagId":5,"sortBy":"title","perPage":20}"#).unwrap();
        println!("group_id={:?} tag_id={:?} sort_by={:?} per_page={:?}", q.group_id, q.tag_id, q.sort_by, q.per_page);
        assert_eq!(q.group_id, Some(3), "groupId 应映射到 group_id");
        assert_eq!(q.tag_id, Some(5), "tagId 应映射到 tag_id");
    }
}
