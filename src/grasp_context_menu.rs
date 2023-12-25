use itertools::Itertools;
use mosaic::{
    capabilities::{
        Archetype, ArchetypeSubject, CollageExportCapability, GroupingCapability, QueueCapability,
        SelectionCapability,
    },
    internals::{
        arrows_from, descriptors_from, gather, par, take_components, targets_from, tiles, MosaicIO,
    },
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::math::Vec2,
    editor_state_machine::{EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    grasp_transitions::QuadtreeUpdateCapability,
    GuiState,
};

impl GraspEditorWindow {
    pub(crate) fn update_context_menu(&mut self, s: &GuiState) {
        if let Some(window_list) = self.window_list.upgrade() {
            if window_list.get_focused() == Some(self)
                && self.rect.contains(s.ui.io().mouse_pos.into())
            {
                if let Some(c) = s.ui.begin_popup("context-menu") {
                    if self.editor_data.selected.is_empty() {
                        println!("DEFAULT MENU IN WINDOW {}", self.window_list_index);
                        if self.show_default_menu(s) {
                            self.trigger(EditorStateTrigger::ExitContextMenu);
                            s.ui.close_current_popup();
                        }
                    } else {
                        println!("SELECTION MENU IN WINDOW {}", self.window_list_index);
                        if self.show_selection_menu(s) {
                            self.trigger(EditorStateTrigger::ExitContextMenu);
                            s.ui.close_current_popup();
                        }
                    }

                    c.end();
                }

                if s.ui.is_mouse_clicked(imgui::MouseButton::Right) {
                    self.trigger(EditorStateTrigger::ClickToContextMenu);
                    s.ui.open_popup("context-menu");
                }
            }
        }
    }

    fn show_selection_menu(&mut self, s: &GuiState) -> bool {
        let queue = self
            .document_mosaic
            .get_all()
            .include_component("NewWindowRequestQueue")
            .get_targets()
            .next()
            .unwrap();

        if s.ui.button("Select") {
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

            return true;
        }

        if s.ui.button("Group - todo") {
            let selection_tile = self.document_mosaic.make_selection();
            if let Some(group) = self
                .document_mosaic
                .get_component(&selection_tile, "GroupOwner")
            {
                let name = group.get("self").as_s32().to_string();
                let members = self
                    .document_mosaic
                    .get_group_members(&name, &selection_tile);
                let c = tiles(members.collect_vec());
                let tile = c.to_tiles(&self.document_mosaic);
                tile.add_component("Label", par(format!("Group: {}", name).as_str()));

                self.document_mosaic.enqueue(&queue, &tile);
                self.document_mosaic.request_quadtree_update();
            }
            return true;
        }

        if s.ui.button("First Neigbours") {
            if let Some(queue) = self
                .document_mosaic
                .get_all()
                .include_component("NewWindowRequestQueue")
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

            return true;
        }

        false
    }

    fn show_default_menu(&mut self, s: &GuiState) -> bool {
        if s.ui.button("Create new node") {
            let pos: Vec2 = s.ui.mouse_pos_on_opening_current_popup().into();
            self.create_new_object(pos - self.editor_data.window_offset - self.editor_data.pan);
            return true;
        }

        if s.ui.button("Toggle debug draw") {
            self.editor_data.debug = !self.editor_data.debug;
            return true;
        }

        false
    }
}
