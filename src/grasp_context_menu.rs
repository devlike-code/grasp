use std::ops::Sub;

use egui::Ui;
use mosaic::{
    capabilities::{CollageExportCapability, QueueCapability},
    internals::{arrows_from, targets_from, tiles, MosaicIO},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    editor_state_machine::{EditorStateTrigger, StateMachine},
    grasp_common::GraspEditorTab,
    grasp_transitions::QuadtreeUpdateCapability,
};

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

    // menu to show when having selection
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
        let resp = ui.button("Create new node");
        if resp.clicked() {
            if let Some(response) = self.response.clone() {
                if let Some(position) = response.interact_pointer_pos() {
                    self.create_new_object(position.sub(self.editor_data.pan));
                    self.document_mosaic.request_quadtree_update();
                }
            }
            self.exit_menu(ui);
        }
    }

    fn exit_menu(&mut self, ui: &mut Ui) {
        ui.close_menu();
        self.trigger(EditorStateTrigger::ExitContextMenu);
    }
}
