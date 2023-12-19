use log::debug;

use crate::{
    core::math::Vec2,
    editor_state_machine::{EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    GuiState,
};

impl GraspEditorWindow {
    pub(crate) fn update_context_menu(&self, s: &GuiState) {
        if let Some(c) = s.ui.begin_popup("context-menu") {
            if self.editor_data.selected.is_empty() {
                if self.show_default_menu(s) {
                    s.ui.close_current_popup();
                }
            } else if self.show_selection_menu(s) {
                s.ui.close_current_popup();
            }

            c.end();
        }

        if s.ui.is_mouse_clicked(imgui::MouseButton::Right) {
            s.ui.open_popup("context-menu");
        }
    }

    fn show_selection_menu(&self, s: &GuiState) -> bool {
        // let queue = self
        //     .document_mosaic
        //     .get_all()
        //     .include_component("NewWindowRequestQueue")
        //     .get_targets()
        //     .next()
        //     .unwrap();
        //
        // ui.menu_button("Filter", |ui| {
        //     if ui.button("Select").clicked() {
        //         let selection_tile = self.document_mosaic.make_selection();
        //         self.document_mosaic
        //             .fill_selection(&selection_tile, &self.editor_data.selected.clone());
        //
        //         let c1 = targets_from(take_components(
        //             &["Group"],
        //             arrows_from(descriptors_from(tiles(vec![selection_tile.clone()]))),
        //         ));
        //
        //         let c2 = arrows_from(targets_from(take_components(
        //             &["Group"],
        //             arrows_from(descriptors_from(tiles(vec![selection_tile.clone()]))),
        //         )));
        //
        //         let c = gather(vec![c1, c2]);
        //         let tile = c.to_tiles(&self.document_mosaic);
        //         tile.add_component("Label", par("Selection"));
        //
        //         self.document_mosaic.enqueue(&queue, &tile);
        //         self.document_mosaic.request_quadtree_update();
        //
        //         self.exit_menu(ui);
        //     }
        //
        //     if ui.button("Group - todo").clicked() {
        //         let selection_tile = self.document_mosaic.make_selection();
        //         if let Some(group) = self
        //             .document_mosaic
        //             .get_component(&selection_tile, "GroupOwner")
        //         {
        //             let name = group.get("self").as_s32().to_string();
        //             let members = self
        //                 .document_mosaic
        //                 .get_group_members(&name, &selection_tile);
        //             let c = tiles(members.collect_vec());
        //             let tile = c.to_tiles(&self.document_mosaic);
        //             tile.add_component("Label", par(format!("Group: {}", name).as_str()));
        //
        //             self.document_mosaic.enqueue(&queue, &tile);
        //             self.document_mosaic.request_quadtree_update();
        //         }
        //         self.exit_menu(ui);
        //     }
        //
        //     if ui.button("First Neigbours").clicked() {
        //         if let Some(queue) = self
        //             .document_mosaic
        //             .get_all()
        //             .include_component("NewWindowRequestQueue")
        //             .get_targets()
        //             .next()
        //         {
        //             // check issue with the vec<&Tile>
        //             let c1 = targets_from(arrows_from(tiles(self.editor_data.selected.clone())));
        //             let c2 = tiles(self.editor_data.selected.clone());
        //             let c3 = arrows_from(tiles(self.editor_data.selected.clone()));
        //
        //             let c = gather(vec![c1, c2, c3]);
        //             let tile = c.to_tiles(&self.document_mosaic);
        //             tile.add_component("Label", par("First Neighbour"));
        //
        //             self.document_mosaic.enqueue(&queue, &tile);
        //             self.document_mosaic.request_quadtree_update();
        //         }
        //
        //         self.exit_menu(ui);
        //     }
        // });
        false
    }

    fn show_default_menu(&self, s: &GuiState) -> bool {
        if s.ui.button("Create new node") {
            let pos = s.ui.mouse_pos_on_opening_current_popup();
            self.create_new_object(Vec2::new(pos[0], pos[1]) - self.editor_data.pan);
            return true;
        }

        false
    }

    fn exit_menu(&mut self, s: &GuiState) {
        self.trigger(EditorStateTrigger::ExitContextMenu);
    }
}
