use std::sync::Arc;

use mosaic::{
    capabilities::{
        process::{self, ProcessCapability},
        ArchetypeSubject,
    },
    internals::{void, MosaicIO},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::{math::Vec2, queues},
    editor_state::windows::GraspEditorWindow,
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_queues::CloseWindowRequestQueue,
    GuiState,
};

impl GraspEditorWindow {
    pub fn context_popup(&mut self, s: &GuiState) {
        if let Some(_token) = s.ui.begin_popup("context-menu") {
            if self.show_default_menu(s) {
                self.trigger(EditorStateTrigger::ExitContextMenu);
            }
        } else if self.state == EditorState::ContextMenu {
            self.trigger(EditorStateTrigger::ExitContextMenu);
        }
    }

    pub(crate) fn update_context_menu(&mut self, front_window_id: Option<usize>, s: &GuiState) {
        if front_window_id == Some(self.window_tile.id)
            && self.rect.contains(s.ui.io().mouse_pos.into())
            && s.ui.is_mouse_clicked(imgui::MouseButton::Right)
        {
            let state_before = self.state;
            self.trigger(EditorStateTrigger::ClickToContextMenu);
            if self.state != state_before && self.state == EditorState::ContextMenu {
                self.editor_data.popup_cursor = s.ui.io().mouse_pos.into();
                s.ui.open_popup("context-menu");
            }
        }
    }

    fn show_selection_menu(&mut self, s: &GuiState) -> bool {
        // let queue = self
        //     .editor_mosaic
        //     .get_all()
        //     .include_component("NewWindowRequestQueue")
        //     .get_targets()
        //     .next()
        //     .unwrap();

        if let Some(_token) = s.ui.begin_menu("Add Component") {
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
        }

        s.ui.separator();

        if let Some(_token) = s.ui.begin_menu("Transformers") {
            if let Some(transformers_tile) = self
                .editor_mosaic
                .get_all()
                .include_component("Transformers")
                .next()
            {
                let transformers = transformers_tile
                    .iter()
                    .get_dependents()
                    .include_component("Transformer");

                for transformer in transformers {
                    let name = transformer.get("display").as_s32().to_string();
                    let func = transformer.get("fn_name").as_s32().to_string();
                    if s.ui.menu_item(name.clone()) {
                        println!("Started {:?} process with {:?} function", name, func);
                        // if let Some(process) = transformer.mosaic.create_process(&name, params).ok() {

                        // }.
                        

                        return true;
                    }
                }
            }
        }

        false
    }

    fn show_default_menu(&mut self, s: &GuiState) -> bool {
        let editor_mosaic = Arc::clone(&self.editor_mosaic);

        if let Some(menu_token) = s.ui.begin_menu("Window") {
            if s.ui.menu_item("Close") {
                let request = editor_mosaic.new_object("CloseWindowRequest", void());
                queues::enqueue(CloseWindowRequestQueue, request);
                menu_token.end();
                return true;
            }

            menu_token.end();
        }

        s.ui.separator();

        if let Some(_menu_token) = s.ui.begin_menu("View") {
            let grid_on = if self.grid_visible { "X" } else { " " };
            let debug_on = if self.editor_data.debug { "X" } else { " " };
            let ruler_on = if self.ruler_visible { "X" } else { " " };

            if s.menu_item(format!("[{}] Toggle Ruler", ruler_on)) {
                self.ruler_visible = !self.ruler_visible;
            }

            if s.menu_item(format!("[{}] Toggle Debug Draw", debug_on)) {
                self.editor_data.debug = !self.editor_data.debug;
            }

            if s.menu_item(format!("[{}] Toggle Grid", grid_on)) {
                self.grid_visible = !self.grid_visible;
            }
        }

        s.ui.separator();
        s.ui.separator();

        if s.ui.menu_item("Create new node") {
            let pos: Vec2 = self.editor_data.popup_cursor;
            self.create_new_object(pos - self.editor_data.window_offset - self.editor_data.pan);
            return true;
        }

        s.ui.separator();

        if !self.editor_data.selected.is_empty() && self.show_selection_menu(s) {
            let previous_selection = self.editor_data.selected.to_owned();
            self.trigger(EditorStateTrigger::ExitContextMenu);

            if self.editor_data.selected.is_empty() {
                self.editor_data.selected = previous_selection;
                self.trigger(EditorStateTrigger::ClickToSelect);
            }

            return true;
        }

        false
    }
}
