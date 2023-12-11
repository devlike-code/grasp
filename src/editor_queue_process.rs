use egui_toast::{Toast, ToastKind, ToastOptions};

use itertools::Itertools;
use mosaic::{
    capabilities::{CollageImportCapability, QueueCapability},
    internals::{void, MosaicIO},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};

use crate::editor_state::GraspEditorState;

impl GraspEditorState {
    pub fn process_requests(&mut self) {
        self.process_toast_queue();
        self.process_new_tab_queue();
        self.process_quadtree_queue();
    }

    fn process_toast_queue(&mut self) {
        while let Some(request) = self.document_mosaic.dequeue(&self.toast_request_queue) {
            let toast_message = request.get("self").as_s32();
            self.toasts.add(Toast {
                text: toast_message.to_string().into(),
                kind: ToastKind::Info,
                options: ToastOptions::default()
                    .duration_in_seconds(5.0)
                    .show_icon(false)
                    .show_progress(true),
            });

            request.iter().delete();
        }
    }

    fn process_new_tab_queue(&mut self) {
        while let Some(request) = self.document_mosaic.dequeue(&self.new_tab_request_queue) {
            let name = request
                .clone()
                .into_iter()
                .get_descriptors()
                .collect_vec()
                .first()
                .unwrap()
                .get("self")
                .as_s32()
                .to_string();

            if let Some(collage) = request.to_collage() {
                let mut tab = self.new_tab(collage);
                tab.name = name.to_string().clone();
                self.dock_state.main_surface_mut().push_to_first_leaf(tab);

                request.iter().delete();
            }
        }
    }

    fn process_quadtree_queue(&mut self) {
        while let Some(request) = self.document_mosaic.dequeue(&self.refresh_quadtree_queue) {
            for tab in self
                .editor_state_tile
                .iter()
                .get_arrows_from()
                .include_component("ToTab")
                .get_targets()
            {
                self.document_mosaic
                    .enqueue(&tab, &self.document_mosaic.new_object("void", void()));

                request.iter().delete();
            }
        }
    }
}
