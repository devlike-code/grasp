use std::collections::VecDeque;
use std::sync::Arc;

use mosaic::internals::Mosaic;

use crate::editor_state::windows::GraspEditorWindow;

// ================= Grasp editor window list ======================
pub struct GraspEditorWindowList {
    pub current_index: u32,
    pub editor_mosaic: Arc<Mosaic>,
    pub windows: VecDeque<GraspEditorWindow>,
    pub named_windows: Vec<String>,
}

impl GraspEditorWindowList {
    pub fn new(mosaic: &Arc<Mosaic>) -> Self {
        Self {
            current_index: Default::default(),
            windows: Default::default(),
            named_windows: Default::default(),
            editor_mosaic: Arc::clone(mosaic),
        }
    }
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
}
