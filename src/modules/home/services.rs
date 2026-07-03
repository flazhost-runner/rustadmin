//! Frontend-template catalog service (opentailwind switcher).
//!
//! Live-first: fetch the opentailwind catalog from the GitHub tree API **once** → cache in
//! memory (TTL 6h) + on disk (`public/fe/templates/_catalog.json`); fall back to a curated
//! static list only when the source is offline. Per-item preview HTML is the **real** template
//! HTML downloaded on-demand from `RawBaseURL/{slug}.html` and cached locally (anti-SSRF: only
//! valid slugs). Server-side search + windowed pagination (12/page) with the active item pinned.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;

use crate::config;
use crate::config::fe_templates::{
    curated, is_valid_slug, FeTemplate, DEFAULT_FE_TEMPLATE, RAW_BASE_URL,
};
use crate::errors::{AppError, AppResult};
use crate::helpers::pagination::{page_window, PageParams, PaginationMeta};

const PER_PAGE: u64 = 12;
const CATALOG_TTL: Duration = Duration::from_secs(6 * 3600);
const TREE_URL: &str =
    "https://api.github.com/repos/lindoai/opentailwind/git/trees/master?recursive=1";

pub struct CatalogPage {
    pub rows: Vec<FeTemplate>,
    pub meta: PaginationMeta,
    pub pages: Vec<Option<u64>>,
}

#[async_trait]
pub trait IFeCatalogService: Send + Sync {
    /// Full catalog (live → disk → curated), active template pinned to the front.
    async fn catalog(&self, active_slug: &str) -> Vec<FeTemplate>;
    /// Distinct categories (for the catalog filter dropdown).
    async fn categories(&self) -> Vec<String>;
    /// Server-side search + pagination (12/page) with the active template pinned to page 1.
    async fn paginate(
        &self,
        q_name: Option<&str>,
        q_category: Option<&str>,
        page: Option<u64>,
        active_slug: &str,
    ) -> CatalogPage;
    /// Proxy a template's **real** preview HTML (local cache → download → cache). Anti-SSRF.
    async fn preview_html(&self, slug: &str) -> AppResult<String>;
    /// Download + cache the selected template (called on Save).
    async fn ensure(&self, slug: &str) -> AppResult<()>;
    /// Active landing HTML: `None` for the pinned default (rendered via the native rich view),
    /// otherwise the real downloaded HTML.
    async fn active_html(&self, slug: &str) -> AppResult<Option<String>>;
}

pub struct FeCatalogService {
    client: reqwest::Client,
    cache: Mutex<Option<(Vec<FeTemplate>, Instant)>>,
}

impl Default for FeCatalogService {
    fn default() -> Self {
        Self::new()
    }
}

impl FeCatalogService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("RustAdmin/0.1 (+frontend-template-switcher)")
            .connect_timeout(Duration::from_secs(4)) // fail fast when offline
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_default();
        Self {
            client,
            cache: Mutex::new(None),
        }
    }

    fn catalog_file() -> PathBuf {
        config::asset("public/fe/templates/_catalog.json")
    }

    fn html_file(slug: &str) -> PathBuf {
        config::asset(&format!("public/fe/templates/{slug}.html"))
    }

    /// Load the catalog with the live → disk → curated fallback (never holds the lock over await).
    async fn load(&self) -> Vec<FeTemplate> {
        // 1. fresh in-memory cache?
        if let Some((list, at)) = self.cache.lock().unwrap().as_ref() {
            if at.elapsed() < CATALOG_TTL {
                return list.clone();
            }
        }
        // 2. live GitHub tree
        if let Some(list) = self.fetch_github().await {
            self.write_disk_catalog(&list);
            *self.cache.lock().unwrap() = Some((list.clone(), Instant::now()));
            return list;
        }
        // 3. disk cache
        if let Some(list) = self.read_disk_catalog() {
            *self.cache.lock().unwrap() = Some((list.clone(), Instant::now()));
            return list;
        }
        // 4. curated fallback
        let list = curated();
        *self.cache.lock().unwrap() = Some((list.clone(), Instant::now()));
        list
    }

    async fn fetch_github(&self) -> Option<Vec<FeTemplate>> {
        let resp = self.client.get(TREE_URL).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let v: Value = resp.json().await.ok()?;
        let arr = v.get("tree")?.as_array()?;
        let mut out = Vec::new();
        for item in arr {
            let Some(path) = item.get("path").and_then(|p| p.as_str()) else {
                continue;
            };
            if let Some(name) = path
                .strip_prefix("landings/")
                .and_then(|p| p.strip_suffix(".html"))
            {
                if !name.contains('/') {
                    if let Some(t) = FeTemplate::from_slug(name) {
                        out.push(t);
                    }
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn write_disk_catalog(&self, list: &[FeTemplate]) {
        if let Ok(json) = serde_json::to_string(list) {
            let p = Self::catalog_file();
            if let Some(dir) = p.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(p, json);
        }
    }

    fn read_disk_catalog(&self) -> Option<Vec<FeTemplate>> {
        let data = fs::read_to_string(Self::catalog_file()).ok()?;
        let list: Vec<FeTemplate> = serde_json::from_str(&data).ok()?;
        if list.is_empty() {
            None
        } else {
            Some(list)
        }
    }
}

#[async_trait]
impl IFeCatalogService for FeCatalogService {
    async fn catalog(&self, active_slug: &str) -> Vec<FeTemplate> {
        let mut list = self.load().await;
        if let Some(pos) = list.iter().position(|t| t.slug == active_slug) {
            let active = list.remove(pos);
            list.insert(0, active);
        }
        list
    }

    async fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.load().await.into_iter().map(|t| t.category).collect();
        cats.sort();
        cats.dedup();
        cats
    }

    async fn paginate(
        &self,
        q_name: Option<&str>,
        q_category: Option<&str>,
        page: Option<u64>,
        active_slug: &str,
    ) -> CatalogPage {
        let mut all = self.catalog(active_slug).await;
        if let Some(n) = q_name.filter(|s| !s.trim().is_empty()) {
            let n = n.to_lowercase();
            all.retain(|t| t.name.to_lowercase().contains(&n) || t.slug.contains(&n));
        }
        if let Some(c) = q_category.filter(|s| !s.trim().is_empty()) {
            let c = c.to_lowercase();
            all.retain(|t| t.category.to_lowercase().contains(&c));
        }
        let params = PageParams::new(page, Some(PER_PAGE), PER_PAGE);
        let total = all.len() as u64;
        let meta = PaginationMeta::new(total, params);
        let start = ((meta.page - 1) * PER_PAGE) as usize;
        let rows: Vec<FeTemplate> = all
            .into_iter()
            .skip(start)
            .take(PER_PAGE as usize)
            .collect();
        let pages = page_window(meta.page, meta.total_pages);
        CatalogPage { rows, meta, pages }
    }

    async fn preview_html(&self, slug: &str) -> AppResult<String> {
        if !is_valid_slug(slug) {
            return Err(AppError::bad_request("Unknown template"));
        }
        // local cache first (instant, network-independent)
        let cache_file = Self::html_file(slug);
        if let Ok(html) = fs::read_to_string(&cache_file) {
            if !html.is_empty() {
                return Ok(html);
            }
        }
        // download the real template HTML, then cache
        let url = format!("{RAW_BASE_URL}/{slug}.html");
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let html = resp
                    .text()
                    .await
                    .map_err(|e| AppError::internal(format!("preview fetch: {e}")))?;
                if let Some(dir) = cache_file.parent() {
                    let _ = fs::create_dir_all(dir);
                }
                let _ = fs::write(&cache_file, &html);
                Ok(html)
            }
            // last resort so the UI isn't broken offline: a self-contained generated preview
            _ => {
                let t = FeTemplate::from_slug(slug)
                    .ok_or_else(|| AppError::bad_request("Invalid template slug"))?;
                Ok(generated_preview(&t))
            }
        }
    }

    async fn ensure(&self, slug: &str) -> AppResult<()> {
        self.preview_html(slug).await.map(|_| ())
    }

    async fn active_html(&self, slug: &str) -> AppResult<Option<String>> {
        if slug == DEFAULT_FE_TEMPLATE {
            return Ok(None); // native rich landing
        }
        Ok(Some(self.preview_html(slug).await?))
    }
}

/// A self-contained themed HTML preview (offline last-resort only).
fn generated_preview(t: &FeTemplate) -> String {
    format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light">
<title>{name}</title><script src="https://cdn.tailwindcss.com"></script></head>
<body class="bg-white text-slate-800">
  <header class="px-8 py-5 flex items-center justify-between border-b">
    <span class="font-bold text-lg text-indigo-600">{name}</span>
    <nav class="space-x-6 text-sm text-slate-600"><a>Home</a><a>About</a><a>Services</a><a>Contact</a></nav>
  </header>
  <section class="px-8 py-20 text-center bg-gradient-to-b from-indigo-50 to-white">
    <p class="uppercase tracking-widest text-indigo-500 text-xs font-semibold">{category}</p>
    <h1 class="text-4xl md:text-5xl font-extrabold mt-3">Beautiful {name}</h1>
    <p class="text-slate-500 mt-4 max-w-xl mx-auto">opentailwind template preview ({slug}).</p>
    <button class="mt-6 px-6 py-3 rounded-lg bg-indigo-600 text-white font-medium">Get Started</button>
  </section>
  <footer class="px-8 py-8 text-center text-slate-400 text-sm border-t">{category} · {slug}</footer>
</body></html>"##,
        name = t.name,
        category = t.category,
        slug = t.slug,
    )
}
