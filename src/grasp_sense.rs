use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::Sub,
};

use imgui::Key;
use itertools::Itertools;
use mosaic::{
    internals::{Tile, TileFieldEmptyQuery, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::{gui::imgui_keys::ExtraKeyEvents, math::Vec2},
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    grasp_editor_window_list::GetWindowFocus,
    grasp_transitions::QuadtreeUpdateCapability,
    utilities::QuadTreeFetch,
    GuiState,
};

use crate::grasp_sense::EditorStateTrigger::*;

pub fn hash_input(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

impl GraspEditorWindow {
    pub fn delete_tiles(&self, _tiles: &[Tile]) {
        let quadtree = self.quadtree.lock().unwrap();
        let _object_to_area = self.object_to_area.lock().unwrap();
        let _under_cursor = quadtree.query(self.build_cursor_area()).collect_vec();

        // DELETE HERE (consider recursive deletion too) -- maybe we could do two passes, one to select everything, and one to delete

        self.document_mosaic.request_quadtree_update();
    }
    pub fn under_cursor(&self) -> Vec<usize> {
        let quadtree = self.quadtree.lock().unwrap();

        quadtree
            .query(self.build_cursor_area())
            .map(|e| *e.value_ref())
            .collect_vec()
    }
    pub fn sense(&mut self, s: &GuiState, caught_events: &mut Vec<u64>) {
        if caught_events.contains(&hash_input("all")) {
            return;
        }
        let under_cursor = self.under_cursor();

        let pos: Vec2 = s.ui.io().mouse_pos.into();

        let is_context = self.state == EditorState::ContextMenu;
        let is_focused = GetWindowFocus(&self.document_mosaic)
            .query()
            .map(|index| index == self.window_tile.id)
            .unwrap_or(false);

        let mouse_in_window = self.rect.contains(pos);

        let clicked_left = !caught_events.contains(&hash_input("click left"))
            && s.ui.is_mouse_clicked(imgui::MouseButton::Left);

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

        if s.ui.is_key_down(Key::Delete) && self.state == EditorState::Idle {
            self.delete_tiles(&self.editor_data.selected);
            self.document_mosaic.request_quadtree_update();
        }

        if double_clicked_left && under_cursor.is_empty() && mouse_in_window {
            //
            caught_events.push(hash_input("double click left"));
            self.trigger(DblClickToCreate);
            //
        }else if self.state == EditorState::PropertyChanging && !is_focused{
            self.trigger(EndDrag);

        } else if double_clicked_left && !under_cursor.is_empty() && is_focused {
            //
            let tile = under_cursor.fetch_tile(&self.document_mosaic);
            if let Some(Value::S32(label)) = tile
                .iter()
                .get_descriptors()
                .include_component("Label")
                .next()
                .map(|tile| tile.get("self"))
            {
                self.editor_data.tile_changing = Some(tile.id);
                self.editor_data.selected = vec![tile];

                self.editor_data.text = label.to_string();
                self.editor_data.previous_text = label.to_string();
                caught_events.push(hash_input("double click left"));
                self.trigger(DblClickToRename);
            }
            //
        } else if clicked_left && under_cursor.is_empty() && mouse_in_window && !is_context {
            //
            caught_events.push(hash_input("click left"));
            self.trigger(ClickToDeselect);
            //
        } else if clicked_left && !under_cursor.is_empty() && mouse_in_window {
            //
            caught_events.push(hash_input("click left"));
            self.editor_data.selected = under_cursor.fetch_tiles(&self.document_mosaic);
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
            self.editor_data.selected = vec![tile_under_mouse];
            caught_events.push(hash_input("start drag left"));
            self.trigger(DragToLink);
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
        } else if start_dragging_left && under_cursor.is_empty() && mouse_in_window && is_focused {
            //
            self.editor_data.selected = vec![];
            self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
            caught_events.push(hash_input("start drag left"));
            self.trigger(DragToSelect);
            //
        } else if start_dragging_middle && mouse_in_window && is_focused {
            //
            caught_events.push(hash_input("start drag middle"));
            self.trigger(DragToPan);
            //
        } else if end_dragging_middle || end_dragging_left {
            //
            self.trigger(EndDrag);
            //
        }

        self.editor_data.cursor = pos.sub(self.editor_data.pan);

        self.left_drag_last_frame = s.ui.is_mouse_dragging(imgui::MouseButton::Left);
        self.middle_drag_last_frame = s.ui.is_mouse_dragging(imgui::MouseButton::Middle);

        // TODO: do we still need this?

        // areas_to_remove.into_iter().for_each(|areas_vec: Vec<u64>| {
        //     for a in areas_vec {
        //         self.quadtree.delete_by_handle(a);
        //     }
        // });
    }
}
