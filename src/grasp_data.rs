use std::sync::Arc;

use egui::{vec2, Vec2};
use grasp::internals::{default_vals, EntityId, Mosaic, MosaicIO, Tile};

#[derive(Debug)]
pub struct GraspMetaData {
    pub mosaic: Arc<Mosaic>,
    pub graps_objects: Vec<GraspObject>,
}

impl Default for GraspMetaData {
    fn default() -> Self {
        Self {
            mosaic: Mosaic::new(),
            graps_objects: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct GraspObject {
    pub tile_data: EntityId,
    pub drag_start: Vec2,
}

impl GraspObject {
    pub fn new(tile: Tile) -> GraspObject {
        let position_default = vec2(100.0, 100.0);
        let drag_start_default = vec2(0.0, 0.0);
        let is_selected_default = false;
        GraspObject {
            tile_data: tile.id,
            drag_start: drag_start_default,
        }
    }
}
