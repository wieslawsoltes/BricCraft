//! Native map presentation contracts. Acquisition and world generation belong to the host.
use ferroui_base::media::IImage;
use std::rc::Rc;

/// Decoded display tiles only; the OSM map image is never used as terrain geometry.
#[derive(Clone)]
pub struct MapTile {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
    pub image: Rc<dyn IImage>,
}
