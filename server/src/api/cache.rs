//! 进程内短 TTL 缓存 + 单飞去重（single-flight）。
//!
//! 用途：详情页对 asmr.one / DLsite 的外网抓取（0.4–0.9s/次）在反复浏览时重复付费。
//! 这里做旁路缓存：命中直接返回，未命中回源并合并同 key 的并发请求；
//! 回源失败写 3 秒负缓存（只挡并发风暴与连点），过后照常重试。
//!
//! TTL 的取舍：asmr.one 的音轨/文件树/详情在一次会话内基本不变，5 分钟足够覆盖
//! 「列表→详情→返回→再进」的高频路径；DLsite 简介与预览同理。内存缓存，重启即清。

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
struct Entry<V> {
    value: Arc<Result<V, String>>,
    expires_at: Instant,
    inserted_at: Instant,
}

/// 条目容量上限：超出时先清过期，仍超则淘汰最早插入的条目。
/// tracks JSON 单条可达几百 KB，无上限的话长会话刷几百个作品会积累几十 MB。
const MAX_ENTRIES: usize = 256;

pub struct TtlCache<V> {
    ttl: Duration,
    map: Mutex<HashMap<String, Entry<V>>>,
    /// 单飞：同 key 的并发回源共享同一次 Future，其余请求订阅 leader 的广播结果。
    inflight: Mutex<HashMap<String, Arc<tokio::sync::broadcast::Sender<Arc<Result<V, String>>>>>>,
}

/// 失败负缓存时长。
const FAILURE_TTL: Duration = Duration::from_secs(3);

impl<V: Clone + Send + Sync + 'static> TtlCache<V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            map: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
        }
    }

    fn get_fresh(&self, key: &str, now: Instant) -> Option<Arc<Result<V, String>>> {
        let mut map = self.map.lock().unwrap();
        match map.get(key) {
            Some(e) if e.expires_at > now => Some(e.value.clone()),
            Some(_) => {
                map.remove(key);
                None
            }
            None => None,
        }
    }

    fn store(&self, key: &str, value: Arc<Result<V, String>>) {
        let ttl = if value.is_ok() { self.ttl } else { FAILURE_TTL };
        let now = Instant::now();
        let mut map = self.map.lock().unwrap();
        map.insert(
            key.to_string(),
            Entry { value, expires_at: now + ttl, inserted_at: now },
        );
        if map.len() > MAX_ENTRIES {
            // 先清过期条目，仍超限则按插入顺序淘汰最旧的
            map.retain(|_, e| e.expires_at > now);
            while map.len() > MAX_ENTRIES {
                let oldest = map
                    .iter()
                    .min_by_key(|(_, e)| e.inserted_at)
                    .map(|(k, _)| k.clone());
                match oldest {
                    Some(k) => {
                        map.remove(&k);
                    }
                    None => break,
                }
            }
        }
    }

    /// 取值：命中新鲜缓存直接返回；否则回源（同 key 并发请求合并为一次）。
    pub async fn get_or_fetch<F, Fut>(&self, key: &str, fetch: F) -> Result<V, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, String>> + Send + 'static,
    {
        if let Some(hit) = self.get_fresh(key, Instant::now()) {
            return hit.as_ref().clone();
        }

        // 认领单飞：拿到未注册 sender 的一方为 leader，负责回源；其余订阅结果。
        let (tx, rx) = {
            let mut inflight = self.inflight.lock().unwrap();
            match inflight.get(key) {
                Some(tx) => (tx.clone(), Some(tx.subscribe())),
                None => {
                    let (tx, _rx) = tokio::sync::broadcast::channel::<Arc<Result<V, String>>>(1);
                    let tx = Arc::new(tx);
                    inflight.insert(key.to_string(), tx.clone());
                    (tx, None)
                }
            }
        };

        let result = match rx {
            // 订阅者：等 leader 的结果；leader 中途取消则退化为自行回源
            Some(mut rx) => match rx.recv().await {
                Ok(v) => v,
                Err(_) => Arc::new(fetch().await),
            },
            // leader：回源并广播
            None => {
                let result = Arc::new(fetch().await);
                self.store(key, result.clone());
                self.inflight.lock().unwrap().remove(key);
                let _ = tx.send(result.clone());
                result
            }
        };
        result.as_ref().clone()
    }
}

/// 全局缓存实例（OnceLock 保证惰性初始化）。
pub mod asmrcache {
    use super::TtlCache;
    use std::sync::OnceLock;
    use std::time::Duration;

    fn cache<V: Clone + Send + Sync + 'static>(ttl: Duration) -> TtlCache<V> {
        TtlCache::new(ttl)
    }

    /// asmr.one 作品详情（按数字 id），5 分钟。
    pub fn work_detail() -> &'static TtlCache<crate::api::asmr_online::AsmrOnlineWork> {
        static C: OnceLock<TtlCache<crate::api::asmr_online::AsmrOnlineWork>> = OnceLock::new();
        C.get_or_init(|| cache(Duration::from_secs(300)))
    }

    /// asmr.one 原始 tracks JSON（音轨与文件树的共同来源），5 分钟。
    pub fn tracks_json() -> &'static TtlCache<serde_json::Value> {
        static C: OnceLock<TtlCache<serde_json::Value>> = OnceLock::new();
        C.get_or_init(|| cache(Duration::from_secs(300)))
    }

    /// DLsite 作品简介（按 RJ 号），10 分钟。
    pub fn dlsite_description() -> &'static TtlCache<String> {
        static C: OnceLock<TtlCache<String>> = OnceLock::new();
        C.get_or_init(|| cache(Duration::from_secs(600)))
    }

    /// DLsite / asmr.one 元数据预览（按 RJ+source），5 分钟。
    pub fn scraped_preview() -> &'static TtlCache<crate::db::models::ScrapedWork> {
        static C: OnceLock<TtlCache<crate::db::models::ScrapedWork>> = OnceLock::new();
        C.get_or_init(|| cache(Duration::from_secs(300)))
    }

    /// DLsite 精确评分（按 RJ 号），10 分钟（评分变化极慢）。
    pub fn dlsite_rating() -> &'static TtlCache<Option<f64>> {
        static C: OnceLock<TtlCache<Option<f64>>> = OnceLock::new();
        C.get_or_init(|| cache(Duration::from_secs(600)))
    }
}

/// DLsite 列表页缓存。
pub mod dlsitecache {
    use super::TtlCache;
    use crate::api::asmr_online::AsmrWorksPage;
    use std::sync::OnceLock;
    use std::time::Duration;

    /// DLsite「最新作品」列表页（按页码），5 分钟（最新列表变化快，但短缓存足够去抖）。
    pub fn latest_page() -> &'static TtlCache<AsmrWorksPage> {
        static C: OnceLock<TtlCache<AsmrWorksPage>> = OnceLock::new();
        C.get_or_init(|| TtlCache::new(Duration::from_secs(300)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn caches_success() {
        let c: TtlCache<u32> = TtlCache::new(Duration::from_secs(60));
        let n = Arc::new(AtomicU32::new(0));
        for _ in 0..3 {
            let n = n.clone();
            let r = c
                .get_or_fetch("k", move || {
                    let n = n.clone();
                    async move {
                        n.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, String>(42)
                    }
                })
                .await
                .unwrap();
            assert_eq!(r, 42);
        }
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failure_is_negative_cached_briefly() {
        let c: TtlCache<u32> = TtlCache::new(Duration::from_secs(60));
        let n = Arc::new(AtomicU32::new(0));
        let f = {
            let n = n.clone();
            move || {
                let n = n.clone();
                async move {
                    n.fetch_add(1, Ordering::SeqCst);
                    Err::<u32, _>("boom".to_string())
                }
            }
        };
        assert!(c.get_or_fetch("k", f.clone()).await.is_err());
        // 负缓存窗口内的第二次调用不再回源
        assert!(c.get_or_fetch("k", f).await.is_err());
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn distinct_keys_distinct_values() {
        let c: TtlCache<u32> = TtlCache::new(Duration::from_secs(60));
        let a = c
            .get_or_fetch("a", || async { Ok::<_, String>(1) })
            .await
            .unwrap();
        let b = c
            .get_or_fetch("b", || async { Ok::<_, String>(2) })
            .await
            .unwrap();
        assert_eq!((a, b), (1, 2));
    }

    #[tokio::test]
    async fn concurrent_calls_share_single_fetch() {
        let c: Arc<TtlCache<u32>> = Arc::new(TtlCache::new(Duration::from_secs(60)));
        let n = Arc::new(AtomicU32::new(0));
        let mut handles = Vec::new();
        for _ in 0..5 {
            let c = c.clone();
            let n = n.clone();
            handles.push(tokio::spawn(async move {
                c.get_or_fetch("k", || {
                    let n = n.clone();
                    async move {
                        n.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        Ok::<_, String>(7)
                    }
                })
                .await
                .unwrap()
            }));
        }
        for h in handles {
            assert_eq!(h.await.unwrap(), 7);
        }
        assert_eq!(n.load(Ordering::SeqCst), 1, "并发请求应只回源一次");
    }

    #[tokio::test]
    async fn evicts_oldest_when_over_capacity() {
        let c: TtlCache<u32> = TtlCache::new(Duration::from_secs(60));
        for i in 0..300u32 {
            c.get_or_fetch(&format!("k{i}"), || async move {
                Ok::<_, String>(i)
            })
            .await
            .unwrap();
        }
        // 最早插入的 44 条应被淘汰（300 - 256），近期条目仍在
        assert!(c.get_fresh("k0", Instant::now()).is_none());
        assert!(c.get_fresh("k299", Instant::now()).is_some());
    }
}
