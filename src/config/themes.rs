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

/// The 9 canonical palettes (exact hex from `@flazhost-nodeadmin/core` THEMES;
/// order = NodeAdmin key order: Blue default first, then alphabetical).
pub const THEMES: &[Theme] = &[
    Theme {
        name: "Blue",
        primary: "#3B82F6",
        secondary: "#60A5FA",
        light: "#DBEAFE",
        dark: "#1E40AF",
    },
    Theme {
        name: "Black",
        primary: "#374151",
        secondary: "#4B5563",
        light: "#6B7280",
        dark: "#1F2937",
    },
    Theme {
        name: "Brown",
        primary: "#A16207",
        secondary: "#D97706",
        light: "#FEF3C7",
        dark: "#78350F",
    },
    Theme {
        name: "Green",
        primary: "#10B981",
        secondary: "#34D399",
        light: "#D1FAE5",
        dark: "#047857",
    },
    Theme {
        name: "Grey",
        primary: "#6B7280",
        secondary: "#9CA3AF",
        light: "#E5E7EB",
        dark: "#374151",
    },
    Theme {
        name: "Orange",
        primary: "#F59E0B",
        secondary: "#FBBF24",
        light: "#FEF3C7",
        dark: "#D97706",
    },
    Theme {
        name: "Purple",
        primary: "#8B5CF6",
        secondary: "#A78BFA",
        light: "#F3E8FF",
        dark: "#6D28D9",
    },
    Theme {
        name: "Red",
        primary: "#EF4444",
        secondary: "#F87171",
        light: "#FECACA",
        dark: "#B91C1C",
    },
    Theme {
        name: "Yellow",
        primary: "#F59E0B",
        secondary: "#FCD34D",
        light: "#FEF3C7",
        dark: "#D97706",
    },
];

/// All theme names in order.
pub fn theme_names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

/// Convenience constant list of names (kept in sync with [`THEMES`]).
pub const THEME_NAMES: &[&str] = &[
    "Blue", "Black", "Brown", "Green", "Grey", "Orange", "Purple", "Red", "Yellow",
];

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
    fn has_nine_canonical_themes() {
        assert_eq!(THEMES.len(), 9);
        assert_eq!(THEME_NAMES.len(), 9);
        assert_eq!(theme_names(), THEME_NAMES.to_vec());
    }

    #[test]
    fn default_is_blue() {
        assert_eq!(DEFAULT_THEME, "Blue");
        assert_eq!(get_theme("Blue").primary, "#3B82F6");
        assert_eq!(get_theme("Blue").light, "#DBEAFE");
        assert_eq!(get_theme("nonexistent").name, "Blue");
        assert_eq!(get_theme("Red").dark, "#B91C1C");
        assert_eq!(get_theme("Yellow").secondary, "#FCD34D");
    }
}
