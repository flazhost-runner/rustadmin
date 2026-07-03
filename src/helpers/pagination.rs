//! Pagination math + metadata (mirrors NodeAdmin `paginate()`), dialect-agnostic.
//!
//! The actual row slicing is done by SeaORM's `Paginator` in services; this module owns
//! the page-param parsing/clamping, the `meta` shape consumed by views/JSON, and the
//! **windowed** page-number list used by the canonical pagination UI (Prev · 1 · … · n · Next).

use serde::Serialize;

/// Allowed page sizes for the `q_page_size` selector (matches the canonical table).
pub const PAGE_SIZES: &[u64] = &[10, 20, 50, 100];

/// Parsed + clamped pagination request parameters.
#[derive(Debug, Clone, Copy)]
pub struct PageParams {
    pub page: u64,
    pub page_size: u64,
}

impl PageParams {
    /// Build from raw optional query values, clamping to sane bounds.
    pub fn new(page: Option<u64>, page_size: Option<u64>, default_size: u64) -> Self {
        let page = page.unwrap_or(1).max(1);
        let mut page_size = page_size.unwrap_or(default_size);
        if !PAGE_SIZES.contains(&page_size) {
            page_size = default_size;
        }
        PageParams { page, page_size }
    }

    /// SeaORM `Paginator` uses 0-based page indexes.
    pub fn zero_based(&self) -> u64 {
        self.page - 1
    }

    /// Row number offset so the "No" column continues across pages.
    pub fn row_offset(&self) -> u64 {
        self.page_size * (self.page - 1)
    }
}

/// Metadata returned alongside paginated data (serialized into views and JSON).
#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
    pub has_prev: bool,
    pub has_next: bool,
    /// 1-based index of the first row on this page (0 when empty).
    pub from: u64,
    /// 1-based index of the last row on this page (0 when empty).
    pub to: u64,
}

impl PaginationMeta {
    pub fn new(total: u64, params: PageParams) -> Self {
        let page_size = params.page_size.max(1);
        let total_pages = total.div_ceil(page_size).max(1);
        let page = params.page.min(total_pages).max(1);
        let from = if total == 0 {
            0
        } else {
            page_size * (page - 1) + 1
        };
        let to = (page_size * page).min(total);
        PaginationMeta {
            total,
            page,
            page_size,
            total_pages,
            has_prev: page > 1,
            has_next: page < total_pages,
            from,
            to,
        }
    }
}

/// Windowed page-number list for the pagination UI: `Prev · 1 · … · cur-2..cur+2 · … · last · Next`.
/// `None` entries are ellipses. Avoids rendering all numbers for large catalogs (≈54 pages).
pub fn page_window(current: u64, total_pages: u64) -> Vec<Option<u64>> {
    let total = total_pages.max(1);
    let cur = current.clamp(1, total);
    let mut pages: Vec<Option<u64>> = Vec::new();
    let mut last_printed = 0u64;

    for p in 1..=total {
        let near_current = p >= cur.saturating_sub(2) && p <= cur + 2;
        let is_edge = p == 1 || p == total;
        if near_current || is_edge {
            if last_printed != 0 && p - last_printed > 1 {
                pages.push(None); // ellipsis
            }
            pages.push(Some(p));
            last_printed = p;
        }
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_page_size() {
        let p = PageParams::new(Some(0), Some(999), 10);
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 10); // 999 not allowed → default
        let p = PageParams::new(None, Some(50), 10);
        assert_eq!(p.page_size, 50);
    }

    #[test]
    fn meta_math() {
        let m = PaginationMeta::new(95, PageParams::new(Some(2), Some(10), 10));
        assert_eq!(m.total_pages, 10);
        assert_eq!(m.from, 11);
        assert_eq!(m.to, 20);
        assert!(m.has_prev && m.has_next);

        let empty = PaginationMeta::new(0, PageParams::new(Some(1), Some(10), 10));
        assert_eq!(empty.total_pages, 1);
        assert_eq!(empty.from, 0);
        assert_eq!(empty.to, 0);
        assert!(!empty.has_prev && !empty.has_next);
    }

    #[test]
    fn windowed_pages() {
        // 54 pages, current 27 → 1 … 25 26 27 28 29 … 54
        let w = page_window(27, 54);
        assert_eq!(w.first(), Some(&Some(1)));
        assert_eq!(w.last(), Some(&Some(54)));
        assert!(w.contains(&None)); // has ellipses
        assert!(w.contains(&Some(27)));
        // small page count → no ellipsis
        let small = page_window(1, 3);
        assert_eq!(small, vec![Some(1), Some(2), Some(3)]);
    }
}
