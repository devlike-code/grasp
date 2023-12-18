use std::ops::Sub;

use imgui::Key;
use itertools::Itertools;
use mosaic::{
    internals::{Tile, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::math::Vec2,
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    grasp_transitions::QuadtreeUpdateCapability,
    utilities::QuadTreeFetch,
    GuiState,
};

use crate::grasp_sense::EditorStateTrigger::*;

impl GraspEditorWindow {
    pub fn delete_tiles(&self, tiles: &Vec<Tile>) {
        let quadtree = self.quadtree.lock().unwrap();
        let mut object_to_area = self.object_to_area.lock().unwrap();

        let under_cursor = quadtree.query(self.build_cursor_area()).collect_vec();

        // DELETE HERE (consider recursive deletion too) -- maybe we could do two passes, one to select everything, and one to delete

        self.document_mosaic.request_quadtree_update();
    }

    pub fn sense(&mut self, s: &GuiState) {
        self.editor_data.cursor_delta = s.ui.mouse_drag_delta().into();

        if let Some(mut rect_delta) = self.editor_data.rect_delta {
            rect_delta += s.ui.mouse_drag_delta().into();
            self.editor_data.rect_delta = Some(rect_delta);
        } else {
            self.editor_data.rect_delta = Some(Vec2::ZERO);
        }

        if s.ui.is_key_down(Key::Delete) && self.state == EditorState::Idle {
            self.delete_tiles(&self.editor_data.selected);
            self.document_mosaic.request_quadtree_update();
        }

        let under_cursor = {
            let quadtree = self.quadtree.lock().unwrap();
            quadtree
                .query(self.build_cursor_area())
                .map(|e| *e.value_ref())
                .collect_vec()
        };

        if s.ui.is_mouse_double_clicked(imgui::MouseButton::Left) && under_cursor.is_empty() {
            //
            self.trigger(DblClickToCreate);
            //
        } else if s.ui.is_mouse_double_clicked(imgui::MouseButton::Left) && !under_cursor.is_empty()
        {
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

                self.trigger(DblClickToRename);
            }
            //
        } else if s.ui.is_mouse_clicked(imgui::MouseButton::Left) && under_cursor.is_empty() {
            //
            self.trigger(ClickToDeselect);
        //
        } else if s.ui.is_mouse_clicked(imgui::MouseButton::Left) && !under_cursor.is_empty() {
            //
            self.editor_data.selected = under_cursor.fetch_tiles(&self.document_mosaic);
            self.trigger(ClickToSelect);
            //
        } else if s.ui.is_mouse_dragging(imgui::MouseButton::Left)
            && !under_cursor.is_empty()
            && (s.ui.is_key_down(Key::LeftAlt) || s.ui.is_key_down(Key::RightAlt))
        {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            self.editor_data.selected = vec![tile_under_mouse];
            self.trigger(DragToLink);
            //
        } else if s.ui.is_mouse_dragging(imgui::MouseButton::Left) && !under_cursor.is_empty() {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            if !self.editor_data.selected.contains(&tile_under_mouse) {
                self.editor_data.selected = vec![tile_under_mouse];
            }
            self.trigger(DragToMove);
            //
        } else if s.ui.is_mouse_dragging(imgui::MouseButton::Left) && under_cursor.is_empty() {
            //
            self.editor_data.selected = vec![];
            self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
            self.trigger(DragToSelect);
            //
        } else if s.ui.is_mouse_dragging(imgui::MouseButton::Middle) {
            //
            self.trigger(DragToPan);
            //
        } else if s.ui.is_mouse_released(imgui::MouseButton::Left)
            || s.ui.is_mouse_released(imgui::MouseButton::Middle)
        {
            //
            self.trigger(EndDrag);
            //
        }

        // TODO: do we still need this?

        // areas_to_remove.into_iter().for_each(|areas_vec: Vec<u64>| {
        //     for a in areas_vec {
        //         self.quadtree.delete_by_handle(a);
        //     }
        // });
    }
}
