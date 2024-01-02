use std::collections::VecDeque;

use crate::grasp_editor_window::GraspEditorWindow;
use crate::GuiState;
use ::mosaic::internals::MosaicIO;
use log::error;
use mosaic::capabilities::QueueCapability;

use mosaic::internals::void;

// ================= Grasp editor window list ======================
#[derive(Default)]
pub struct GraspEditorWindowList {
    pub current_index: u32,
    pub windows: VecDeque<GraspEditorWindow>,
    pub named_windows: Vec<String>,
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

    pub fn get_position_by_name(&self, name: &str) -> Option<usize> {
        self.windows.iter().position(|w| w.name.as_str() == name)
    }

    pub fn show(&mut self, s: &GuiState) {
        let mut caught_events = vec![];

        for window in &mut self.windows {
            window.show(s, &mut caught_events);
        }

        caught_events.clear();
    }

    pub fn request_focus(&self, name: &str) {
        if let Some(index) = self.get_position_by_name(name) {
            let window = self.windows.get(index).unwrap();
            let mosaic = &window.get_editor_mosaic();
            let request = mosaic.new_object("FocusWindowRequest", void());
            mosaic.enqueue(&window.window_tile, &request);
        } else {
            error!("CANNOT FIND WINDOW NAME {}", name);
        }
    }
}
