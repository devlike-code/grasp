use std::ops::Sub;

use egui::Ui;
use mosaic::{
    capabilities::{
        ArchetypeSubject, CollageExportCapability, QueueCapability, SelectionCapability,
    },
    internals::{
        arrows_from, descriptors_from, gather, par, take_components, targets_from, tiles, MosaicIO,
    },
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
        ui.menu_button("Filter", |ui| {
            if ui.button("Select").clicked() {
                if let Some(queue) = self
                    .document_mosaic
                    .get_all()
                    .include_component("NewTabRequestQueue")
                    .get_targets()
                    .next()
                {
                    let selection_tile = self.document_mosaic.make_selection();
                    self.document_mosaic
                        .fill_selection(&selection_tile, &self.editor_data.selected.clone());

                    let c1 = targets_from(take_components(
                        &["Group"],
                        arrows_from(descriptors_from(tiles(vec![selection_tile.clone()]))),
                    ));

                    let c2 = arrows_from(targets_from(take_components(
                        &["Group"],
                        arrows_from(descriptors_from(tiles(vec![selection_tile.clone()]))),
                    )));

                    let c = gather(vec![c1, c2]);
                    let tile = c.to_tiles(&self.document_mosaic);
                    tile.add_component("Label", par("Selection"));

                    self.document_mosaic.enqueue(&queue, &tile);
                    self.document_mosaic.request_quadtree_update();
                }

                self.exit_menu(ui);
            }

            if ui.button("Group - todo").clicked() {
                self.exit_menu(ui);
            }

            if ui.button("First Neigbours").clicked() {
                if let Some(queue) = self
                    .document_mosaic
                    .get_all()
                    .include_component("NewTabRequestQueue")
                    .get_targets()
                    .next()
                {
                    // check issue with the vec<&Tile>
                    let c1 = targets_from(arrows_from(tiles(self.editor_data.selected.clone())));
                    let c2 = tiles(self.editor_data.selected.clone());
                    let c3 = arrows_from(tiles(self.editor_data.selected.clone()));

                    let c = gather(vec![c1, c2, c3]);
                    let tile = c.to_tiles(&self.document_mosaic);
                    tile.add_component("Label", par("First Neighbour"));

                    self.document_mosaic.enqueue(&queue, &tile);
                    self.document_mosaic.request_quadtree_update();
                }

                self.exit_menu(ui);
            }
        });
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
