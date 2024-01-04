use std::thread::panicking;

use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{void, MosaicIO},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    editor_state::windows::GraspEditorWindow,
    editor_state_machine::{EditorStateTrigger, StateMachine},
    GuiState,
};

impl GraspEditorWindow {
    pub fn context_popup(&mut self, s: &GuiState) {
        s.ui.popup("context-menu", || {
            if self.editor_data.selected.is_empty() {
                if self.show_default_menu(s) {
                    self.trigger(EditorStateTrigger::ExitContextMenu);
                    s.ui.close_current_popup();
                }
            } else if self.show_selection_menu(s) {
                self.trigger(EditorStateTrigger::ExitContextMenu);
                s.ui.close_current_popup();
            }
        });
    }

    pub(crate) fn update_context_menu(&mut self, front_window_id: Option<usize>, s: &GuiState) {
        if front_window_id == Some(self.window_tile.id)
            && self.rect.contains(s.ui.io().mouse_pos.into())
            && s.ui.is_mouse_clicked(imgui::MouseButton::Right)
        {
            self.trigger(EditorStateTrigger::ClickToContextMenu);
            s.ui.open_popup("context-menu");
        }
    }

    fn show_selection_menu(&mut self, s: &GuiState) -> bool {
        let queue = self
            .editor_mosaic
            .get_all()
            .include_component("NewWindowRequestQueue")
            .get_targets()
            .next()
            .unwrap();

        if let Some(token) = s.ui.begin_menu("Add Component") {
            if let Some(category_set) = self
                .editor_mosaic
                .get_all()
                .include_component("ComponentCategorySet")
                .next()
            {
                let categories = category_set
                    .iter()
                    .get_dependents()
                    .include_component("ComponentCategory");

                for category in categories {
                    if !category.get("hidden").as_bool() {
                        if let Some(token) =
                            s.ui.begin_menu(category.get("name").as_s32().to_string())
                        {
                            for item in category
                                .iter()
                                .get_dependents()
                                .include_component("ComponentEntry")
                            {
                                if !item.get("hidden").as_bool()
                                    && s.ui.menu_item(item.get("display").as_s32().to_string())
                                {
                                    for s in &self.editor_data.selected {
                                        s.add_component(
                                            &item.get("name").as_s32().to_string(),
                                            void(),
                                        );
                                    }

                                    return true;
                                }
                            }

                            token.end();
                        }
                    }
                }
            } else {
                panic!("No category set!");
            }

            token.end();
        }

        // s.ui.separator();

        // if s.ui.button("Select") {
        //     let selection_tile = self.document_mosaic.make_selection();
        //     self.document_mosaic
        //         .fill_selection(&selection_tile, &self.editor_data.selected.clone());

        //     let c1 = targets_from(take_components(
        //         &["Group"],
        //         arrows_from(descriptors_from(tiles(vec![selection_tile.clone()]))),
        //     ));

        //     let c2 = arrows_from(targets_from(take_components(
        //         &["Group"],
        //         arrows_from(descriptors_from(tiles(vec![selection_tile.clone()]))),
        //     )));

        //     let c = gather(vec![c1, c2]);
        //     let tile = c.to_tiles(&self.document_mosaic);
        //     tile.add_component("Label", par("Selection"));

        //     self.document_mosaic.enqueue(&queue, &tile);
        //     self.document_mosaic.request_quadtree_update();

        //     return true;
        // }

        // if s.ui.button("Group - todo") {
        //     let selection_tile = self.document_mosaic.make_selection();
        //     if let Some(group) = self
        //         .document_mosaic
        //         .get_component(&selection_tile, "GroupOwner")
        //     {
        //         let name = group.get("self").as_s32().to_string();
        //         let members = self
        //             .document_mosaic
        //             .get_group_members(&name, &selection_tile);
        //         let c = tiles(members.collect_vec());
        //         let tile = c.to_tiles(&self.document_mosaic);
        //         tile.add_component("Label", par(format!("Group: {}", name).as_str()));

        //         self.document_mosaic.enqueue(&queue, &tile);
        //         self.document_mosaic.request_quadtree_update();
        //     }
        //     return true;
        // }

        // if s.ui.button("First Neigbours") {
        //     if let Some(queue) = self
        //         .document_mosaic
        //         .get_all()
        //         .include_component("NewWindowRequestQueue")
        //         .get_targets()
        //         .next()
        //     {
        //         let c1 = targets_from(arrows_from(tiles(self.editor_data.selected.clone())));
        //         let c2 = tiles(self.editor_data.selected.clone());
        //         let c3 = arrows_from(tiles(self.editor_data.selected.clone()));

        //         let c = gather(vec![c1, c2, c3]);
        //         let tile = c.to_tiles(&self.document_mosaic);
        //         tile.add_component("Label", par("First Neighbour"));

        //         self.document_mosaic.enqueue(&queue, &tile);
        //         self.document_mosaic.request_quadtree_update();
        //     }

        //     return true;
        // }

        false
    }

    fn show_default_menu(&mut self, s: &GuiState) -> bool {
        true
        // let editor_state = self.get_editor_state();
        // let editor_mosaic = &editor_state.editor_mosaic;

        // if s.ui.button("Create new node") {
        //     let pos: Vec2 = s.ui.mouse_pos_on_opening_current_popup().into();
        //     self.create_new_object(pos - self.editor_data.window_offset - self.editor_data.pan);

        //     return true;
        // }

        // if s.ui.button("Toggle debug draw") {
        //     self.editor_data.debug = !self.editor_data.debug;
        //     return true;
        // }

        // if s.ui.button("Close Window") {
        //     let request = editor_mosaic.new_object("CloseWindowRequest", void());
        //     queues::enqueue(CloseWindowRequestQueue, request);
        //     return true;
        // }
        // false
    }
}
