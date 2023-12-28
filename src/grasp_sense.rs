use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use imgui::Key;
use itertools::Itertools;
use mosaic::{
    internals::{MosaicCRUD, MosaicIO, Tile, TileFieldEmptyQuery, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::{
        gui::imgui_keys::ExtraKeyEvents,
        math::{Rect2, Vec2},
    },
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    grasp_editor_window_list::*,
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
        let mut object_to_area = self.object_to_area.lock().unwrap();
        let _under_cursor = quadtree.query(self.build_cursor_area()).collect_vec();
        let mut areas_to_remove: Vec<u64> = vec![];

        for selected in &self.editor_data.selected {
            self.document_mosaic.delete_tile(selected.id);
            if let Some(area_id) = object_to_area.get(&selected.id) {
                areas_to_remove.push(*area_id);
                object_to_area.remove(&selected.id);
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
    pub fn sense(&mut self, s: &GuiState, caught_events: &mut Vec<u64>) {
        fn trigget_rename(
            window: &mut GraspEditorWindow,
            tile: Tile,
            caught_events: &mut Vec<u64>,
        ) {
            if let Some(Value::S32(label)) = tile
                .iter()
                .get_descriptors()
                .include_component("Label")
                .next()
                .map(|tile| tile.get("self"))
            {
                window.editor_data.tile_changing = Some(tile.id);
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
            .map(|t| t.component.is("Label"))
            .unwrap_or(false);
        let pos: Vec2 = s.ui.io().mouse_pos.into();

        let is_focused = GetWindowFocus(&self.document_mosaic)
            .query()
            .map(|index| index == self.window_tile.id)
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

        if s.ui.is_key_down(Key::Delete) && self.state == EditorState::Idle {
            self.delete_tiles(&self.editor_data.selected);
            self.editor_data.selected.clear();
            self.document_mosaic.request_quadtree_update();
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
            trigget_rename(self, tile, caught_events);
            //
        } else if double_clicked_left && !under_cursor.is_empty() && is_focused {
            //
            let tile = under_cursor.fetch_tile(&self.document_mosaic);
            trigget_rename(self, tile, caught_events);
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
            //println!("CLICK MIDDLE");
            if !is_focused {
                self.set_focus();
            }
            caught_events.push(hash_input("click middle"));
            self.trigger(ClickToDeselect);
        } else if clicked_right && under_cursor.is_empty() && mouse_in_window {
            //
            //println!("CLICK RIGHT");
            if !is_focused {
                self.set_focus();
            }
            caught_events.push(hash_input("click right"));

            self.trigger(EndDrag);

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
        //
        {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            self.editor_data.selected = vec![tile_under_mouse];
            caught_events.push(hash_input("start drag left"));
            self.trigger(DragToLink);
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
                self.set_focus();
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
