use itertools::Itertools;
use mosaic::{
    capabilities::{
        Archetype, ArchetypeSubject, CollageExportCapability, GroupingCapability, QueueCapability,
        SelectionCapability,
    },
    internals::{
        arrows_from, descriptors_from, gather, par, take_components, targets_from, tiles, void,
        MosaicIO,
    },
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::{math::Vec2, queues},
    editor_state_machine::{EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    grasp_queues::CloseWindowRequestQueue,
    grasp_transitions::QuadtreeUpdateCapability,
    GuiState,
};

impl GraspEditorWindow {
    pub fn context_popup(&mut self, s: &GuiState) {
        s.ui.popup("context-menu", || {
            if self.editor_data.selected.is_empty() {
                // println!("DEFAULT MENU IN WINDOW {}", self.window_list_index);
                if self.show_default_menu(s) {
                    self.trigger(EditorStateTrigger::ExitContextMenu);
                    s.ui.close_current_popup();
                }
            } else {
                //  println!("SELECTION MENU IN WINDOW {}", self.window_list_index);
                if self.show_selection_menu(s) {
                    self.trigger(EditorStateTrigger::ExitContextMenu);
                    s.ui.close_current_popup();
                }
            }
        });
    }

    pub(crate) fn update_context_menu(&mut self, s: &GuiState) {
        let window_list = &self.get_editor_state().window_list;
        if window_list.get_focused() == Some(self)
            && self.rect.contains(s.ui.io().mouse_pos.into())
            && s.ui.is_mouse_clicked(imgui::MouseButton::Right)
        {
            self.trigger(EditorStateTrigger::ClickToContextMenu);
            s.ui.open_popup("context-menu");
        }
    }

    fn show_selection_menu(&mut self, s: &GuiState) -> bool {
        let editor_state = self.get_editor_state();
        let editor_mosaic = &editor_state.editor_mosaic;

        // let queue = editor_mosaic
        //     .get_all()
        //     .include_component("NewWindowRequestQueue")
        //     .get_targets()
        //     .next()
        //     .unwrap();

        if let Some(token) = s.ui.begin_menu("Add Component") {
            let categories = editor_state.loaded_categories.clone();

            for category in &categories {
                if !category.hidden {
                    if let Some(token) = s.ui.begin_menu(category.name.clone()) {
                        for item in &category.components {
                            if !item.hidden && s.ui.menu_item(item.display.clone()) {
                                for s in &self.editor_data.selected {
                                    s.add_component(&item.name, void());
                                }

                                return true;
                            }
                        }

                        token.end();
                    }
                }
            }

            token.end();
        }

        s.ui.separator();

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
        let editor_state = self.get_editor_state();
        let editor_mosaic = &editor_state.editor_mosaic;

        if s.ui.button("Create new node") {
            let pos: Vec2 = s.ui.mouse_pos_on_opening_current_popup().into();
            self.create_new_object(pos - self.editor_data.window_offset - self.editor_data.pan);

            return true;
        }

        if s.ui.button("Toggle debug draw") {
            self.editor_data.debug = !self.editor_data.debug;
            return true;
        }

        if s.ui.button("Close Window") {
            let request = editor_mosaic.new_object("CloseWindowRequest", void());
            queues::enqueue(CloseWindowRequestQueue, request);
            return true;
        }
        false
    }
}
