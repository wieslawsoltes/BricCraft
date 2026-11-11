//! Dimensional construction parts. One world unit is one stud; a plate is 0.4 units.
//! Catalog identifiers are stable save-format identifiers, never list indices.
#![forbid(unsafe_code)]

mod catalog;
mod connectors;
mod palette;
mod types;

pub use catalog::CATALOG;
pub use connectors::{Connector, ConnectorKind};
pub use palette::{COLOR_NAMES, PALETTE};
pub use types::{Category, Part, Shape};

pub const STUD_PITCH_MM: f32 = 8.0;
pub const PLATE_HEIGHT: f32 = 0.4;
pub const STUD_RADIUS: f32 = 0.3;
pub const STUD_HEIGHT: f32 = 0.18;
pub const BRICK_SEAM: f32 = 0.022;

pub fn part(id: u16) -> Option<&'static Part> {
    CATALOG.iter().find(|part| part.id == id)
}

pub fn search(query: &str, category: Option<Category>) -> Vec<&'static Part> {
    let query = query.to_ascii_lowercase();
    CATALOG
        .iter()
        .filter(|part| {
            category.is_none_or(|category| category == part.category)
                && (query.is_empty()
                    || part.name.to_ascii_lowercase().contains(&query)
                    || part.id.to_string() == query)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_ids_and_dimensions_are_valid() {
        let mut ids = HashSet::new();
        for part in CATALOG {
            assert!(ids.insert(part.id), "duplicate catalog id {}", part.id);
            assert!(part.size.iter().all(|v| *v > 0));
            assert_eq!(part.rotated_size(0), part.rotated_size(2));
            assert_eq!(part.rotated_size(1), part.rotated_size(3));
        }
        assert!(CATALOG.len() >= 100);
        assert_eq!(PALETTE.len(), COLOR_NAMES.len());
    }

    #[test]
    fn search_has_stable_ids() {
        assert!(search("axle", Some(Category::Technic))
            .iter()
            .all(|p| p.category == Category::Technic));
        assert_eq!(part(3).unwrap().size, [2, 3, 4]);
    }
}
