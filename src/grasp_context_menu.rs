use std::ops::Sub;

use egui::Ui;
use mosaic::{
    capabilities::{CollageExportCapability, QueueCapability},
    internals::{arrows_from, targets_from, tiles, MosaicIO},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::grasp_common::GraspEditorTab;

impl GraspEditorTab {
    pub(crate) fn update_context_menu(&mut self, _ui: &mut Ui) {
        if let Some(response) = self.response.clone() {
            response.context_menu(|ui| {
                if !self.editor_data.selected.is_empty() {
                    self.show_selection_menu(ui);
                } else {
                    self.show_default_menu(ui);
                }
            });
        }
    }

    // menu to show when havng selection
    fn show_selection_menu(&mut self, ui: &mut Ui) {
        if ui.button("Filter: My Neighbors").clicked() {
            if let Some(queue) = self
                .document_mosaic
                .get_all()
                .include_component("NewTabRequestQueue")
                .get_targets()
                .next()
            {
                self.document_mosaic.enqueue(
                    &queue,
                    &targets_from(arrows_from(tiles(self.editor_data.selected.clone())))
                        .to_tiles(&self.document_mosaic),
                );
            }
            ui.close_menu();
        }
    }

    // default context menu
    fn show_default_menu(&mut self, ui: &mut Ui) {
        if ui.button("Create new node").clicked() {
            if let Some(response) = self.response.clone() {
                let position = response
                    .interact_pointer_pos()
                    .unwrap()
                    .sub(self.editor_data.pan);
                self.create_new_object(position);
                ui.close_menu();
            }
        }
    }
}
