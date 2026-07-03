//! Admin theme palettes for the theme switcher (DB-driven, no rebuild).
//!
//! Byte-identical to NodeAdmin `@flazhost-nodeadmin/core` `THEMES`. One set of views is
//! driven by 4 colours (primary/secondary/light/dark) via CSS variables + an inline
//! Tailwind config in the layout head. Active theme = `settings.theme` (default Blue).

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Theme {
    pub name: &'static str,
    pub primary: &'static str,
    pub secondary: &'static str,
    pub light: &'static str,
    pub dark: &'static str,
}

pub const DEFAULT_THEME: &str = "Blue";

/// The 5 standard palettes (Blue/Purple/Green/Orange/Red — identical to NodeAdmin).
pub const THEMES: &[Theme] = &[
    Theme {
        name: "Blue",
        primary: "#3B82F6",
        secondary: "#60A5FA",
        light: "#EFF6FF",
        dark: "#1E40AF",
    },
    Theme {
        name: "Purple",
        primary: "#8B5CF6",
        secondary: "#A78BFA",
        light: "#F5F3FF",
        dark: "#5B21B6",
    },
    Theme {
        name: "Green",
        primary: "#10B981",
        secondary: "#34D399",
        light: "#ECFDF5",
        dark: "#065F46",
    },
    Theme {
        name: "Orange",
        primary: "#F59E0B",
        secondary: "#FCD34D",
        light: "#FFFBEB",
        dark: "#92400E",
    },
    Theme {
        name: "Red",
        primary: "#EF4444",
        secondary: "#F87171",
        light: "#FEF2F2",
        dark: "#991B1B",
    },
];

/// All theme names in order.
pub fn theme_names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

/// Convenience constant list of names (kept in sync with [`THEMES`]).
pub const THEME_NAMES: &[&str] = &["Blue", "Purple", "Green", "Orange", "Red"];

/// Resolve a palette by name, falling back to the default theme.
pub fn get_theme(name: &str) -> Theme {
    THEMES
        .iter()
        .find(|t| t.name == name)
        .copied()
        .unwrap_or(THEMES[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_five_themes() {
        assert_eq!(THEMES.len(), 5);
        assert_eq!(THEME_NAMES.len(), 5);
        assert_eq!(theme_names(), THEME_NAMES.to_vec());
    }

    #[test]
    fn default_is_blue() {
        assert_eq!(DEFAULT_THEME, "Blue");
        assert_eq!(get_theme("Blue").primary, "#3B82F6");
        assert_eq!(get_theme("nonexistent").name, "Blue");
        assert_eq!(get_theme("Red").dark, "#991B1B");
    }
}
