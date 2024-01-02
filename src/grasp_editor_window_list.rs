use std::collections::VecDeque;
use std::sync::Arc;

use crate::grasp_editor_window::GraspEditorWindow;
use crate::GuiState;
use ::mosaic::internals::MosaicIO;
use log::error;
use mosaic::capabilities::QueueCapability;

use mosaic::internals::{void, Mosaic, TileFieldEmptyQuery, TileFieldSetter};
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
    pub windows: VecDeque<GraspEditorWindow>,
}

impl GraspEditorWindowList {
    pub fn increment(&mut self) -> u32 {
        self.current_index += 1;
        self.current_index
    }

    #[allow(dead_code)]
    pub fn get_focused(&self) -> Option<&GraspEditorWindow> {
        self.windows.front()
    }

    pub fn get_focused_mut(&mut self) -> Option<&mut GraspEditorWindow> {
        self.windows.front_mut()
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

        for window in &mut self.windows {
            window.show(s, &mut caught_events);
        }

        caught_events.clear();
    }

    pub fn focus(&self, name: &str) {
        if let Some(index) = self.windows.iter().position(|w| w.name.as_str() == name) {
            let window = self.windows.get(index).unwrap();
            let mosaic = &window.get_editor_mosaic();
            let request = mosaic.new_object("FocusWindowRequest", void());
            mosaic.enqueue(&window.window_tile, &request);
        } else {
            error!("CANNOT FIND WINDOW NAME {}", name);
        }
    }
}
