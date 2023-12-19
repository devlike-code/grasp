use std::collections::VecDeque;
use std::sync::Mutex;

use crate::core::math::vec2::Vec2;
use crate::grasp_editor_window::GraspEditorWindow;
use crate::GuiState;
use ::mosaic::internals::{MosaicIO, Tile, Value};
use log::debug;
use mosaic::capabilities::{ArchetypeSubject, QueueCapability};

use mosaic::internals::{void, TileFieldQuery};

#[derive(Default)]
pub struct GraspEditorWindowList {
    pub current_index: u32,
    pub windows: Vec<GraspEditorWindow>,
    pub depth_sorted_by_index: Mutex<VecDeque<usize>>,
}

impl GraspEditorWindowList {
    pub fn increment(&mut self) -> u32 {
        self.current_index += 1;
        self.current_index
    }

    pub fn show(&mut self, s: &GuiState) {
        let mut caught_events = vec![];

        let depth_sorted = self.depth_sorted_by_index.lock().unwrap();
        depth_sorted.iter().rev().for_each(|window_id| {
            let window = self.windows.get_mut(*window_id).unwrap();
            window.show(s, &mut caught_events);
        });

        caught_events.clear();
    }

    // TODO: never gets called
    pub fn focus(&self, name: &str) {
        if let Some(pos) = self.windows.iter().position(|w| w.name.as_str() == name) {
            let window = self.windows.get(pos).unwrap();
            let request = window.document_mosaic.new_object("void", void());
            window
                .document_mosaic
                .enqueue(&window.window_tile, &request);
            let id = window.window_tile.id;

            let mut depth = self.depth_sorted_by_index.lock().unwrap();
            if let Some(pos) = depth.iter().position(|p| *p == id) {
                depth.remove(pos);
                depth.push_front(id);
                debug!("{}: REMOVED FROM {} AND PUSHED FORWARD", id, pos);
            }
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
