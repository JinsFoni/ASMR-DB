use serde::{Deserialize, Serialize};

/// 作品（对应 works 表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Work {
    pub id: i64,
    pub rj_code: String,
    pub title_ja: Option<String>,
    pub title_zh: Option<String>,
    pub title_en: Option<String>,
    pub circle_name: Option<String>,
    pub circle_id: Option<String>,
    pub cover_url: Option<String>,
    pub work_type: String,
    pub price: Option<i64>,
    pub sale_date: Option<String>,
    pub description: Option<String>,
    pub age_class: Option<String>,
    pub duration_min: Option<i64>,
    pub file_size_mb: Option<f64>,
    pub dlsite_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 作品 + 派生字段（列表/详情展示用）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkView {
    pub work: Work,
    pub tags: Vec<Tag>,
    pub actors: Vec<String>,
    pub download_status: String,
    pub local_path: Option<String>,
    pub track_count: i64,
    pub total_duration_sec: f64,
    pub group: Option<Group>,
}

/// 分组（对应 groups 表，一个作品属于一个分组）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// 标签（对应 tags 表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub is_preset: bool,
}

/// 声优（对应 voice_actors 表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceActor {
    pub id: i64,
    pub name: String,
    pub dlsite_url: Option<String>,
}

/// 字幕信息（在线或关联字幕）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubtitleInfo {
    pub title: String,
    pub extension: String,
    pub url: String,
}

/// 音轨（对应 audio_tracks 表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Track {
    pub id: i64,
    pub work_id: i64,
    pub file_path: String,
    pub track_number: Option<i64>,
    pub title: Option<String>,
    pub duration_sec: Option<f64>,
    pub file_format: Option<String>,
    pub file_size: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtitles: Vec<SubtitleInfo>,
}

/// 本地文件（对应 local_files 表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalFile {
    pub id: i64,
    pub work_id: i64,
    pub local_path: String,
    pub file_type: String,
    pub download_status: String,
    pub download_progress: f64,
    pub file_size: Option<i64>,
}

/// 播放进度（对应 play_history 表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayHistory {
    pub id: i64,
    pub work_id: i64,
    pub track_id: Option<i64>,
    pub last_position: f64,
    pub play_count: i64,
    pub last_played_at: Option<String>,
}

/// 抓取的 DLsite 元数据（前端 import 时预览用）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapedWork {
    pub rj_code: String,
    pub title_ja: Option<String>,
    pub title_zh: Option<String>,
    pub title_en: Option<String>,
    pub circle_name: Option<String>,
    pub circle_id: Option<String>,
    pub cover_url: Option<String>,
    pub work_type: String,
    pub price: Option<i64>,
    pub sale_date: Option<String>,
    pub description: Option<String>,
    pub age_class: Option<String>,
    pub duration_min: Option<i64>,
    pub file_size_mb: Option<f64>,
    pub dlsite_url: Option<String>,
    pub actors: Vec<String>,
    pub tags: Vec<String>,
}

/// 下载任务（前端下载队列展示用）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DownloadTask {
    pub rj_code: String,
    pub title: String,
    pub status: String, // queued | downloading | paused | done | error
    pub progress: f64,
    pub speed: f64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WorkView 必须序列化为 { "work": { ... }, ... }，前端依赖嵌套的 work 对象。
    #[test]
    fn work_view_serializes_nested_work() {
        let view = WorkView {
            work: Work {
                id: 1,
                rj_code: "RJ01014447".into(),
                ..Default::default()
            },
            tags: vec![],
            actors: vec!["佐倉綾音".into()],
            download_status: "not_downloaded".into(),
            local_path: None,
            track_count: 0,
            total_duration_sec: 0.0,
            group: None,
        };
        let json = serde_json::to_value(&view).unwrap();
        assert!(json.get("work").is_some(), "必须包含嵌套 work 对象");
        assert_eq!(json["work"]["rj_code"], "RJ01014447");
        assert_eq!(json["actors"][0], "佐倉綾音");
        assert!(json.get("id").is_none(), "work 字段不应平铺在顶层");
    }
}
