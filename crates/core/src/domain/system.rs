use serde::{Deserialize, Serialize};

use super::ids::SystemId;

/// How a system's games were physically sold. Drives case shape and, later,
/// which artwork providers and label kinds make sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Cartridge,
    Disc,
    Jewel,
    Bluray,
    Dvd,
    Card,
    Download,
}

impl MediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cartridge => "cartridge",
            Self::Disc => "disc",
            Self::Jewel => "jewel",
            Self::Bluray => "bluray",
            Self::Dvd => "dvd",
            Self::Card => "card",
            Self::Download => "download",
        }
    }
}

/// A console platform.
///
/// `box_aspect` and `box_depth` are real measurements of retail packaging, and
/// they are load-bearing: the artwork pipeline fetches at this aspect and the
/// shelf renders a case at this aspect, so a wrong value distorts every case for
/// the system. They are curated, never inferred.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct System {
    pub id: SystemId,
    pub name: String,
    pub short_name: String,
    pub manufacturer: Option<String>,
    pub generation: Option<i32>,
    /// Width divided by height. CD jewel ~0.95, DVD ~0.70, Switch ~0.62.
    pub box_aspect: f32,
    /// Thickness divided by height.
    pub box_depth: f32,
    pub media_kind: MediaKind,
    /// Hex colour used for synthesised spines, so a shelf reads as grouped.
    pub accent: String,
    pub retroarch_core: Option<String>,
    pub sort_index: i32,
}

impl System {
    /// Guards against a row that would render as a degenerate case.
    pub fn geometry_is_usable(&self) -> bool {
        self.box_aspect.is_finite()
            && self.box_aspect > 0.0
            && self.box_depth.is_finite()
            && self.box_depth > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system(aspect: f32, depth: f32) -> System {
        System {
            id: SystemId::from_db("ps2"),
            name: "PlayStation 2".into(),
            short_name: "PS2".into(),
            manufacturer: Some("Sony".into()),
            generation: Some(6),
            box_aspect: aspect,
            box_depth: depth,
            media_kind: MediaKind::Dvd,
            accent: "#2F4858".into(),
            retroarch_core: Some("pcsx2".into()),
            sort_index: 10,
        }
    }

    #[test]
    fn real_geometry_is_usable() {
        assert!(system(0.7, 0.14).geometry_is_usable());
    }

    #[test]
    fn degenerate_geometry_is_rejected() {
        assert!(!system(0.0, 0.14).geometry_is_usable());
        assert!(!system(0.7, 0.0).geometry_is_usable());
        assert!(!system(f32::NAN, 0.14).geometry_is_usable());
    }
}
