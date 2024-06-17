mod autocache_responder;
pub mod autocacher;
pub use autocache_responder::autocache_responder;
pub use autocacher::AutoCacher;

use moka::future::Cache;
use moka::future::CacheBuilder;
type PageCacheData = (HeaderMap, String);
type PageCache = Cache<String, PageCacheData>;
fn build_cache() -> PageCache {
    CacheBuilder::new(1000)
        .time_to_live(std::time::Duration::from_secs(30))
        .build()
}

use once_cell::sync::Lazy;
use reqwest::header::HeaderMap;
pub static CACHE: Lazy<PageCache> = Lazy::new(build_cache);
pub fn cache_key(session_id: &str, page_url: &str) -> String {
    let url = reqwest::Url::parse(page_url).expect("Failed to parse cache key URL");
    let url_normalized = url.to_string();
    format!("{session_id}@{url_normalized}")
}
