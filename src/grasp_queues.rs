use mosaic::{
    capabilities::CollageImportCapability,
    internals::{void, MosaicIO, Tile},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};
use std::vec::IntoIter;

use grasp_proc_macros::GraspQueue;

use crate::{
    core::queues::{self, dequeue, GraspQueue},
    editor_state::GraspEditorState,
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
        self.editor_state_tile
            .iter()
            .get_arrows_from()
            .include_component("ToTab")
            .get_targets()
    }

    pub fn process_requests(&mut self) {
        self.process_toast_queue();
        self.process_new_tab_queue();
        self.process_quadtree_queue();
    }

    fn process_toast_queue(&mut self) {
        // while let Some(request) = self.document_mosaic.dequeue(&self.toast_request_queue) {
        //     let toast_message = request.get("self").as_s32();
        //     self.toasts.add(Toast {
        //         text: toast_message.to_string().into(),
        //         kind: ToastKind::Info,
        //         options: ToastOptions::default()
        //             .duration_in_seconds(5.0)
        //             .show_icon(false)
        //             .show_progress(true),
        //     });

        //     request.iter().delete();
        // }
    }

    fn process_new_tab_queue(&mut self) {
        while let Some(request) = queues::dequeue(NewWindowRequestQueue, &self.document_mosaic) {
            if let Some(collage) = request.to_collage() {
                self.new_window(collage);
                request.iter().delete();
            }
        }
    }

    fn process_quadtree_queue(&mut self) {
        while let Some(request) = dequeue(QuadtreeUpdateRequestQueue, &self.document_mosaic) {
            for tab in self.iter_all_windows() {
                queues::enqueue_direct(tab, self.document_mosaic.new_object("void", void()))
            }
            request.iter().delete();
        }
    }
}
