use crate::core::math::vec2::Vec2;
use crate::grasp_editor_window::GraspEditorWindow;
use crate::GuiState;
use ::mosaic::internals::{MosaicIO, Tile, Value};
use mosaic::capabilities::{ArchetypeSubject, QueueCapability};

use mosaic::internals::{void, TileFieldQuery};

#[derive(Default)]
pub struct GraspEditorWindowList {
    pub current_index: u32,
    pub windows: Vec<GraspEditorWindow>,
}

impl GraspEditorWindowList {
    pub fn increment(&mut self) -> u32 {
        self.current_index += 1;
        self.current_index
    }

    pub fn show(&mut self, s: &GuiState) {
        for window in &mut self.windows {
            window.show(s);
        }
    }

    pub fn focus(&self, name: &str) {
        if let Some(pos) = self.windows.iter().position(|w| w.name.as_str() == name) {
            let window = self.windows.get(pos).unwrap();
            let request = window.document_mosaic.new_object("void", void());
            window
                .document_mosaic
                .enqueue(&window.window_tile, &request);
        }
    }
}

pub fn get_pos_from_tile(tile: &Tile) -> Option<Vec2> {
    if let Some(tile_pos_component) = tile.get_component("Position") {
        if let (Value::F32(x), Value::F32(y)) = tile_pos_component.get_by(("x", "y")) {
            Some(Vec2::new(x, y))
        } else {
            None
        }
    } else {
        None
    }
}
