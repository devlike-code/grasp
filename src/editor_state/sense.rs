use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use imgui::Key;
use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{pars, void, ComponentValuesBuilderSetter, MosaicIO, Tile, Value},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};

use crate::{
    core::{
        gui::imgui_keys::ExtraKeyEvents,
        math::{Rect2, Vec2},
        structures::enqueue,
    },
    editor_state::helpers::RequireWindowFocus,
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_queues::WindowTileDeleteReactionRequestQueue,
    transformers::{find_selection_owner, select},
    utilities::QuadTreeFetch,
    GuiState,
};

use crate::editor_state::sense::EditorStateTrigger::*;

use super::windows::GraspEditorWindow;

pub fn hash_input(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

impl GraspEditorWindow {
    pub fn delete_tiles(&self, tiles: &[Tile]) {
        if tiles.is_empty() {
            return;
        }

        for selected in tiles {
            self.delete_tiles(selected.iter().get_extensions().as_slice());

            for (name, property) in selected.get_full_archetype() {
                for desc in property {
                    enqueue(
                        WindowTileDeleteReactionRequestQueue,
                        self.editor_mosaic.new_object(
                            "WindowTileDeleteReactionRequest",
                            pars()
                                .set("window", self.window_tile.id as u64)
                                .set("tile", desc.id as u64)
                                .set("component", name.as_str())
                                .ok(),
                        ),
                    );
                }
            }
        }
    }

    pub fn under_cursor(&self) -> Vec<usize> {
        let quadtree = self.quadtree.lock().unwrap();

        quadtree
            .query(self.build_cursor_area())
            .map(|e| *e.value_ref())
            .collect_vec()
    }

    pub fn sense(
        &mut self,
        s: &GuiState,
        front_window_id: Option<usize>,
        caught_events: &mut Vec<u64>,
        properties_hovered: bool,
    ) {
        fn trigger_rename(
            window: &mut GraspEditorWindow,
            tile: Tile,
            comp: String,
            caught_events: &mut Vec<u64>,
        ) {
            if let Some((id, Value::S32(label))) = tile
                .iter()
                .get_descriptors()
                .include_component(&comp)
                .next()
                .map(|tile| (tile.id, tile.get("self")))
            {
                window.editor_data.tile_changing = Some(id);
                window.editor_data.selected = vec![tile];

                window.editor_data.text = label.to_string();
                window.editor_data.previous_text = label.to_string();
                caught_events.push(hash_input("double click left"));
                window.trigger(DblClickToRename);
            }
        }

        if caught_events.contains(&hash_input("all")) {
            return;
        }

        let under_cursor = self.under_cursor();
        let is_label_region = under_cursor
            .first()
            .and_then(|f| self.document_mosaic.get(*f))
            .map(|t| t.component.is("Label") || t.component.is("HasComponent"))
            .unwrap_or(false);
        let pos: Vec2 = s.ui.io().mouse_pos.into();

        let is_focused = front_window_id
            .map(|window| window == self.window_tile.id)
            .unwrap_or(false);
        let mouse_in_window = self.rect.contains(pos);
        let is_resizing = {
            let size = 12.0;
            let lower_left_rect = Rect2::from_pos_size(
                self.rect.max() - [size, size].into(),
                [2.0 * size, 2.0 * size].into(),
            );
            lower_left_rect.contains(pos)
        };

        let clicked_left = !caught_events.contains(&hash_input("click left"))
            && s.ui.is_mouse_clicked(imgui::MouseButton::Left);

        let clicked_middle = !caught_events.contains(&hash_input("click middle"))
            && s.ui.is_mouse_clicked(imgui::MouseButton::Middle);

        let clicked_right = !caught_events.contains(&hash_input("click right"))
            && s.ui.is_mouse_clicked(imgui::MouseButton::Right);

        let double_clicked_left = !caught_events.contains(&hash_input("double click left"))
            && s.ui.is_mouse_double_clicked(imgui::MouseButton::Left);

        let start_dragging_left = !caught_events.contains(&hash_input("start drag left"))
            && !self.left_drag_last_frame
            && s.ui.is_mouse_dragging(imgui::MouseButton::Left)
            && self.state == EditorState::Idle;

        let start_dragging_middle = !caught_events.contains(&hash_input("start drag middle"))
            && !self.middle_drag_last_frame
            && s.ui.is_mouse_dragging(imgui::MouseButton::Middle)
            && self.state == EditorState::Idle;

        let end_dragging_middle =
            s.ui.is_mouse_released(imgui::MouseButton::Middle) && self.state.uses_dragging();
        let end_dragging_left =
            s.ui.is_mouse_released(imgui::MouseButton::Left) && self.state.uses_dragging();

        if !caught_events.contains(&hash_input("left drag")) {
            self.editor_data.cursor_delta = s.ui.io().mouse_delta.into();
        }

        if let Some(mut rect_delta) = self.editor_data.rect_delta {
            rect_delta = s.ui.mouse_drag_delta().into();
            self.editor_data.rect_delta = Some(rect_delta);
        } else {
            self.editor_data.rect_delta = Some(Vec2::ZERO);
        }

        for (key, value) in &[
            (30_usize, "Pick1".to_string()), // Alpha1-5
            (31_usize, "Pick2".to_string()),
            (32_usize, "Pick3".to_string()),
            (33_usize, "Pick4".to_string()),
            (34_usize, "Pick5".to_string()),
        ] {
            if s.ui.io().keys_down[*key]
                && s.ui.io().key_ctrl
                && !self.editor_data.selected.is_empty()
                && is_focused
            {
                let count_unselected = self
                    .editor_data
                    .selected
                    .clone()
                    .iter()
                    .filter(|t| t.get_component("Selected").is_none())
                    .count();

                if count_unselected == self.editor_data.selected.len() {
                    let selected = self.editor_data.selected.clone();
                    let any = self.editor_data.selected.first().cloned().unwrap();
                    select(self, s, &selected, &any);
                }

                if let Some(sel) = self
                    .editor_data
                    .selected
                    .first()
                    .unwrap()
                    .get_component("Selected")
                {
                    if let Some(owner) = find_selection_owner(&sel) {
                        if let Some(old) = self
                            .document_mosaic
                            .get_all()
                            .include_component(value)
                            .next()
                        {
                            if old.target() == owner.0 {
                                return;
                            }

                            if let Some(old_desc) = self
                                .document_mosaic
                                .get_all()
                                .include_component(value)
                                .next()
                            {
                                old_desc.iter().delete();
                            }
                        }

                        owner.0.add_component(value, void());
                    }
                }
            }
        }

        if s.ui.is_key_pressed(Key::Escape) && is_focused {
            for pick in &["Pick1", "Pick2", "Pick3", "Pick4", "Pick5"] {
                self.document_mosaic
                    .get_all()
                    .include_component(pick)
                    .delete();
            }
        }

        if s.ui.is_key_down(Key::Delete)
            && self.state == EditorState::Idle
            && is_focused
            && !properties_hovered
        {
            self.delete_tiles(&self.editor_data.selected);
            self.editor_data.selected.clear();
            self.request_quadtree_update();
        }

        if double_clicked_left && under_cursor.is_empty() && mouse_in_window && is_focused {
            //
            caught_events.push(hash_input("double click left"));
            self.trigger(DblClickToCreate);
            //
        } else if self.state == EditorState::PropertyChanging && !is_focused {
            self.trigger(EndDrag);
        } else if double_clicked_left && !under_cursor.is_empty() && is_focused && is_label_region {
            //
            let tile = under_cursor.fetch_tile(&self.document_mosaic).target();
            let comp = under_cursor
                .fetch_tile(&self.document_mosaic)
                .component
                .to_string();
            trigger_rename(self, tile, comp, caught_events);
            //
        } else if double_clicked_left && !under_cursor.is_empty() && is_focused {
            //
            let tile = under_cursor.fetch_tile(&self.document_mosaic);
            let comp = under_cursor
                .fetch_tile(&self.document_mosaic)
                .component
                .to_string();
            trigger_rename(self, tile, comp, caught_events);
            //
        } else if clicked_left && under_cursor.is_empty() && mouse_in_window
        //&& !is_context
        {
            //
            caught_events.push(hash_input("click left"));
            self.trigger(ClickToDeselect);
            //
        } else if clicked_middle && under_cursor.is_empty() && mouse_in_window {
            //
            if !is_focused {
                self.require_window_focus(self.window_tile.clone());
            }
            caught_events.push(hash_input("click middle"));
            self.trigger(ClickToDeselect);
            //
        } else if clicked_right && !under_cursor.is_empty() && mouse_in_window {
            //
            println!("Not empty right");
            if !is_focused {
                self.require_window_focus(self.window_tile.clone());
            }
            caught_events.push(hash_input("click right"));
            self.editor_data.selected = under_cursor.fetch_tiles(&self.document_mosaic);
            self.trigger(ClickToSelect);
            //
        } else if clicked_right && under_cursor.is_empty() && mouse_in_window {
            //
            if !is_focused {
                self.require_window_focus(self.window_tile.clone());
            }
            caught_events.push(hash_input("click right"));

            self.trigger(EndDrag);

            //
        } else if clicked_left
            && !under_cursor.is_empty()
            && mouse_in_window
            && !self.editor_data.selected.contains(
                &self
                    .document_mosaic
                    .get(*under_cursor.first().unwrap())
                    .unwrap(),
            )
        {
            //
            caught_events.push(hash_input("click left"));
            self.editor_data.selected.clear();
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            self.editor_data.selected.push(tile_under_mouse.clone());
            self.trigger(ClickToSelect);
            //
        } else if start_dragging_left
            && !under_cursor.is_empty()
            && (s.ui.is_modkey_down(Key::LeftAlt) || s.ui.is_modkey_down(Key::RightAlt))
            && mouse_in_window
            && is_focused
        {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            self.editor_data.selected = vec![tile_under_mouse.clone()];
            caught_events.push(hash_input("start drag left"));

            if tile_under_mouse.is_arrow() || tile_under_mouse.is_object() {
                self.trigger(DragToLink);
            } else {
                self.trigger(DragToMove);
            }
            //
        } else if start_dragging_left
            && !under_cursor.is_empty()
            && mouse_in_window
            && is_focused
            && is_label_region
        {
            //
            self.trigger(DragToMove);
            //
        } else if start_dragging_left && !under_cursor.is_empty() && mouse_in_window && is_focused {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);

            if !self.editor_data.selected.contains(&tile_under_mouse) {
                self.editor_data.selected = vec![tile_under_mouse];
            }

            caught_events.push(hash_input("start drag left"));
            self.trigger(DragToMove);
            //
        } else if start_dragging_left && under_cursor.is_empty() && is_resizing && is_focused {
            //
            self.trigger(DragToWindowResize);
            //
        } else if start_dragging_left && under_cursor.is_empty() && mouse_in_window && is_focused {
            //
            self.editor_data.selected = vec![];
            self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
            caught_events.push(hash_input("start drag left"));
            self.trigger(DragToSelect);
            //
        } else if start_dragging_middle && mouse_in_window {
            if !is_focused {
                self.require_window_focus(self.window_tile.clone());
            }

            caught_events.push(hash_input("start drag middle"));

            self.trigger(DragToPan);
            //
        } else if end_dragging_middle || end_dragging_left {
            //
            self.trigger(EndDrag);
            //
        }

        self.editor_data.cursor = pos;

        self.left_drag_last_frame = s.ui.is_mouse_dragging(imgui::MouseButton::Left);
        self.middle_drag_last_frame = s.ui.is_mouse_dragging(imgui::MouseButton::Middle);
    }
}
