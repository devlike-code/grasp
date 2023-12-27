use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::core::math::vec2::Vec2;
use crate::grasp_editor_window::GraspEditorWindow;
use crate::GuiState;
use ::mosaic::internals::{MosaicIO, Tile, Value};
use itertools::Itertools;
use log::error;
use mosaic::capabilities::{ArchetypeSubject, QueueCapability};

use mosaic::internals::{void, Mosaic, TileFieldEmptyQuery, TileFieldQuery, TileFieldSetter};
use mosaic::iterators::component_selectors::ComponentSelectors;

// ================= Window focus helpers for getting and setting value quickly ======================
pub struct GetWindowFocus<'a>(pub &'a Arc<Mosaic>);

impl<'a> TileFieldEmptyQuery for GetWindowFocus<'a> {
    type Output = Option<usize>;

    fn query(&self) -> Self::Output {
        self.0
            .get_all()
            .include_component("EditorStateFocusedWindow")
            .map(|focus| focus.get("self").as_u64() as usize)
            .next()
    }
}

pub struct SetWindowFocus<'a>(pub &'a Arc<Mosaic>, pub usize);

impl<'a> TileFieldEmptyQuery for SetWindowFocus<'a> {
    type Output = ();

    fn query(&self) -> Self::Output {
        for mut focus in self
            .0
            .get_all()
            .include_component("EditorStateFocusedWindow")
        {
            focus.set("self", self.1 as u64);
        }
    }
}

// ================= Grasp editor window list ======================
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

    #[allow(dead_code)]
    pub fn get_focused(&self) -> Option<&GraspEditorWindow> {
        self.depth_sorted_by_index
            .lock()
            .unwrap()
            .front()
            .and_then(|index| self.windows.get(*index))
    }

    pub fn get_focused_mut(&mut self) -> Option<&mut GraspEditorWindow> {
        self.depth_sorted_by_index
            .lock()
            .unwrap()
            .front()
            .and_then(|index| self.windows.get_mut(*index))
    }

    pub fn get_position(&mut self, focused_index: Option<usize>) -> Option<&mut GraspEditorWindow> {
        focused_index.and_then(|focused_index| {
            self.windows
                .iter_mut()
                .find(|w| w.window_tile.id == focused_index)
        })
    }

    pub fn show(&mut self, s: &GuiState) {
        let mut caught_events = vec![];

        let depth_sorted = {
            let vec_deque = self.depth_sorted_by_index.lock().unwrap().clone();
            vec_deque.iter().cloned().collect_vec()
        };

        for window_id in depth_sorted {
            let window = self.windows.get_mut(window_id).unwrap();
            window.show(s, &mut caught_events);
        }

        caught_events.clear();
    }

    pub fn focus(&self, name: &str) {
        if let Some(index) = self.windows.iter().position(|w| w.name.as_str() == name) {
            let window = self.windows.get(index).unwrap();
            let mosaic = &window.document_mosaic;
            let request = mosaic.new_object("FocusWindowRequest", void());
            mosaic.enqueue(&window.window_tile, &request);

            let mut depth = self.depth_sorted_by_index.lock().unwrap();
            if let Some(pos) = depth.iter().position(|p| *p == window.window_list_index) {
                depth.remove(pos);
                depth.push_front(window.window_list_index);
            }
        } else {
            error!("CANNOT FIND WINDOW NAME {}", name);
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
