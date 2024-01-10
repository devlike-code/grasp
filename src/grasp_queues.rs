use grasp_proc_macros::GraspQueue;
use mosaic::{
    capabilities::CollageImportCapability,
    internals::{void, MosaicIO, Tile},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};
use std::vec::IntoIter;

use crate::{
    core::{
        gui::windowing::gui_set_window_focus,
        queues::{self, dequeue, GraspQueue},
    },
    editor_state::foundation::GraspEditorState,
};

#[derive(GraspQueue)]
pub struct NewWindowRequestQueue;

#[derive(GraspQueue)]
pub struct NamedFocusWindowRequestQueue;

#[derive(GraspQueue)]
pub struct CloseWindowRequestQueue;

#[derive(GraspQueue)]
pub struct QuadtreeUpdateRequestQueue;

#[derive(GraspQueue)]
pub struct WindowMessageInboxQueue(Tile);

impl GraspEditorState {
    fn iter_all_windows(&self) -> IntoIter<Tile> {
        //each window tile has arrow "DirectWindowRequest" pointing to "Queue" tile that has descriptor "EditorWindowQueue" attached, and descriptors
        self.editor_state_tile
            .iter()
            .get_arrows_from()
            .include_component("DirectWindowRequest")
            .get_targets()
    }

    //processing all queues on Editor level
    pub fn process_requests(&mut self) {
        self.process_named_focus_window_queue();
        self.process_new_window_queue();
        self.process_quadtree_queue();
        self.process_close_window_queue();
    }

    fn process_named_focus_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(NamedFocusWindowRequestQueue, &self.editor_mosaic)
        {
            let data = request.get("self").as_s32();

            if let Some(pos) = self
                .window_list
                .windows
                .iter()
                .position(|w| w.name == data.to_string())
            {
                let window = self.window_list.windows.remove(pos).unwrap();
                self.window_list.windows.push_front(window);
                gui_set_window_focus(&data.to_string());
            }
        }
    }

    fn process_new_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(NewWindowRequestQueue, &self.editor_mosaic) {
            // TODO: reconnect collage, but with reconstruction into other mosaic
            if let Some(_collage) = request.to_collage() {
                self.new_window(None);
                request.iter().delete();
            }
        }
    }

    fn process_close_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(CloseWindowRequestQueue, &self.editor_mosaic) {
            self.close_window(self.window_list.get_focused().unwrap().window_tile.clone());
            request.iter().delete();
        }
    }
    fn process_quadtree_queue(&mut self) {
        while let Some(request) = dequeue(QuadtreeUpdateRequestQueue, &self.editor_mosaic) {
            let all_window_queues = self.iter_all_windows();
            for window_queue in all_window_queues {
                queues::enqueue_direct(
                    window_queue,
                    self.editor_mosaic
                        .new_object("QuadtreeUpdateRequest", void()),
                )
            }
            request.iter().delete();
        }
    }
}
