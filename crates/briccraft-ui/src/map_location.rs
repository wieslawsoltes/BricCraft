//! Geographic view state without a rendering or platform dependency.
#[derive(Clone, Debug, PartialEq)]
pub struct MapView {
    pub latitude: f64,
    pub longitude: f64,
    pub zoom: u8,
    pub selected_latitude: f64,
    pub selected_longitude: f64,
    pub label: String,
    pub radius: f64,
    pub meters_per_stud: f64,
    pub terrain: bool,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            latitude: 52.516275,
            longitude: 13.377704,
            zoom: 16,
            selected_latitude: 52.516275,
            selected_longitude: 13.377704,
            label: "Berlin · Brandenburg Gate".into(),
            radius: 160.0,
            meters_per_stud: 4.0,
            terrain: true,
        }
    }
}

pub const MAP_PRESETS: [(&str, f64, f64); 4] = [
    ("Berlin", 52.516275, 13.377704),
    ("London", 51.500729, -0.124625),
    ("Kraków", 50.061430, 19.936580),
    ("San Francisco", 37.802400, -122.405800),
];
