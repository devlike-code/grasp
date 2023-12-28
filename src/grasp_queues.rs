use grasp_proc_macros::GraspQueue;
use mosaic::{
    capabilities::CollageImportCapability,
    internals::{void, MosaicIO, Tile},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};
use notify_rust::Notification;
use std::vec::IntoIter;

use crate::{
    core::queues::{self, dequeue, GraspQueue},
    grasp_editor_state::GraspEditorState,
};

#[derive(GraspQueue)]
pub struct ToastRequestQueue;

#[derive(GraspQueue)]
pub struct NewWindowRequestQueue;

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
        self.process_toast_queue();
        self.process_new_window_queue();
        self.process_quadtree_queue();
    }

    fn process_toast_queue(&mut self) {
        while let Some(request) = queues::dequeue(ToastRequestQueue, &self.document_mosaic) {
            let toast_message = request.get("self").as_s32();
            println!("TOAST RECEIVED! {}", toast_message);

            Notification::new()
                .summary("Grasp")
                .body(toast_message.to_string().as_str())
                //.icon("")
                .show()
                .ok();

            request.iter().delete();
        }
    }

    fn process_new_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(NewWindowRequestQueue, &self.document_mosaic) {
            if let Some(collage) = request.to_collage() {
                self.new_window(collage);
                request.iter().delete();
            }
        }
    }

    fn process_quadtree_queue(&mut self) {
        while let Some(request) = dequeue(QuadtreeUpdateRequestQueue, &self.document_mosaic) {
            let all_window_queues = self.iter_all_windows();
            for window_queue in all_window_queues {
                queues::enqueue_direct(
                    window_queue,
                    self.document_mosaic
                        .new_object("QuadtreeUpdateRequest", void()),
                )
            }
            request.iter().delete();
        }
    }
}
