//! Frontend-template (landing) catalog metadata.
//!
//! Mirrors NodeAdmin `src/config/feTemplates.ts`. The live catalog (≈640 items) is fetched
//! from the opentailwind GitHub tree API by the `home` module (Phase 7) and cached; this
//! file holds the **curated fallback** used when the source is offline, plus slug parsing
//! (anti-SSRF) and the pinned default template.
//!
//! Slug shape (opentailwind): `{category}-{NNN}-{name}`, regex
//! `^([a-z]+(?:-[a-z]+)*)-(\d{3})-([a-z0-9-]+)$`. We parse it manually to avoid a regex dep.

use serde::{Deserialize, Serialize};

/// The one template bundled & rendered via a native rich view (PORTING_GUIDE: must be a real
/// opentailwind slug, not a generic "default"). It is also the initial `settings.fe_template`.
pub const DEFAULT_FE_TEMPLATE: &str = "agency-consulting-002-creative-agency";

/// Raw HTML base URL for on-demand template downloads (opentailwind: branch `master`,
/// directory `landings`, file `{slug}.html`).
pub const RAW_BASE_URL: &str =
    "https://raw.githubusercontent.com/lindoai/opentailwind/master/landings";

/// A catalog entry (slug + derived metadata).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeTemplate {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub number: String,
}

impl FeTemplate {
    /// Build an entry from a slug, deriving name/category. Returns `None` for invalid slugs.
    pub fn from_slug(slug: &str) -> Option<Self> {
        let (category, number, name) = parse_slug(slug)?;
        Some(FeTemplate {
            slug: slug.to_string(),
            name: titleize(&name),
            category: titleize(&category),
            number,
        })
    }
}

/// Curated fallback catalog (15 slugs, identical to NodeAdmin FE_TEMPLATES /
/// GoAdmin `curated`; default first) — used when the live opentailwind fetch fails.
pub fn curated() -> Vec<FeTemplate> {
    [
        DEFAULT_FE_TEMPLATE,
        "agency-consulting-001-digital-marketing-agency",
        "technology-saas-001-hero-focused-conversion-page",
        "technology-saas-002-feature-rich-multi-section",
        "ecommerce-retail-001-fashion-boutique",
        "ecommerce-retail-002-luxury-fashion-brand",
        "portfolio-creative-001-creative-portfolio",
        "portfolio-creative-002-minimal-portfolio",
        "professional-services-001-law-firm",
        "real-estate-property-001-real-estate-agency",
        "food-hospitality-001-fine-dining-restaurant",
        "healthcare-wellness-001-family-doctor-clinic",
        "education-training-001-private-school",
        "fitness-sports-001-fitness-center",
        "travel-tourism-001-travel-agency",
    ]
    .iter()
    .filter_map(|s| FeTemplate::from_slug(s))
    .collect()
}

/// Validate a slug strictly (anti-SSRF: only well-formed slugs may be fetched).
pub fn is_valid_slug(slug: &str) -> bool {
    parse_slug(slug).is_some()
}

/// Parse `{category}-{NNN}-{name}` → (category, NNN, name).
///
/// `category` = one-or-more lowercase-alpha words joined by `-`; `NNN` = exactly 3 digits;
/// `name` = `[a-z0-9-]+`.
fn parse_slug(slug: &str) -> Option<(String, String, String)> {
    if slug.is_empty() || slug.len() > 120 {
        return None;
    }
    let parts: Vec<&str> = slug.split('-').collect();
    // Find the segment that is exactly 3 digits — the boundary between category and name.
    let idx = parts
        .iter()
        .position(|p| p.len() == 3 && p.chars().all(|c| c.is_ascii_digit()))?;
    if idx == 0 || idx == parts.len() - 1 {
        return None; // need at least one category word before and one name word after
    }
    let category_parts = &parts[..idx];
    let number = parts[idx];
    let name_parts = &parts[idx + 1..];

    // category words must be lowercase alpha only
    if !category_parts
        .iter()
        .all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase()))
    {
        return None;
    }
    // name words must be [a-z0-9]+
    if !name_parts.iter().all(|w| {
        !w.is_empty()
            && w.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    }) {
        return None;
    }

    Some((
        category_parts.join("-"),
        number.to_string(),
        name_parts.join("-"),
    ))
}

/// `creative-agency` → `Creative Agency`.
fn titleize(s: &str) -> String {
    s.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_slug_is_valid() {
        assert!(is_valid_slug(DEFAULT_FE_TEMPLATE));
        let t = FeTemplate::from_slug(DEFAULT_FE_TEMPLATE).unwrap();
        assert_eq!(t.category, "Agency Consulting");
        assert_eq!(t.number, "002");
        assert_eq!(t.name, "Creative Agency");
    }

    #[test]
    fn rejects_bad_slugs() {
        assert!(!is_valid_slug("no-number-here"));
        assert!(!is_valid_slug("../etc/passwd"));
        assert!(!is_valid_slug("123-456-789"));
        assert!(!is_valid_slug("agency-12-name")); // not 3 digits
        assert!(!is_valid_slug("002-name")); // no category
        assert!(!is_valid_slug("category-002")); // no name
        assert!(!is_valid_slug("Agency-002-name")); // uppercase category
    }

    #[test]
    fn curated_nonempty_and_contains_default() {
        let c = curated();
        assert!(!c.is_empty());
        assert!(c.iter().any(|t| t.slug == DEFAULT_FE_TEMPLATE));
    }
}
