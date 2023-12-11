use std::ops::Sub;

use egui::{Response, Sense, Ui, Vec2};
use itertools::Itertools;
use mosaic::{
    internals::{MosaicCRUD, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{GraspEditorTab, QuadTreeFetch, UiKeyDownExtract}, grasp_transitions::QuadtreeUpdateCapability,
};

impl GraspEditorTab {
    fn sense_begin_frame(&mut self, ui: &mut Ui) -> Response {
        let (resp, _) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        if let Some(pos) = resp.hover_pos() {
            self.editor_data.cursor = pos.sub(self.editor_data.pan);
        }

        self.editor_data.cursor_delta = resp.drag_delta();

        if let Some(mut rect_delta) = self.editor_data.rect_delta {
            rect_delta += resp.drag_delta();
            self.editor_data.rect_delta = Some(rect_delta);
        } else {
            self.editor_data.rect_delta = Some(Vec2::ZERO);
        }

        resp
    }

    pub fn sense(&mut self, ui: &mut Ui) {
        use egui::PointerButton::*;
        use EditorStateTrigger::*;

        let mouse = self.sense_begin_frame(ui);
        let under_cursor = self.quadtree.query(self.build_cursor_area()).collect_vec();
        let mut areas_to_remove = vec![];

        if ui.delete_down() && self.state == EditorState::Idle {
            for selected in &self.editor_data.selected {
                self.document_mosaic.delete_tile(selected.id);
                if let Some(area_id) = self.object_to_area.get(&selected.id) {
                    areas_to_remove.push(area_id.clone());
                    self.object_to_area.remove(&selected.id);
                }
            }
            self.editor_data.selected.clear();

            //self.update_quadtree(None);
            self.document_mosaic.request_quadtree_update();
        }

        if mouse.double_clicked() && under_cursor.is_empty() {
            //
            self.trigger(DblClickToCreate);
            //
        } else if mouse.double_clicked() && !under_cursor.is_empty() {
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
        } else if mouse.clicked() && under_cursor.is_empty() {
            //
            self.trigger(ClickToDeselect);
            //
        } else if mouse.clicked() && !under_cursor.is_empty() {
            //
            self.editor_data.selected = under_cursor.fetch_tiles(&self.document_mosaic);
            self.trigger(ClickToSelect);
            //
        } else if mouse.drag_started_by(Primary) && !under_cursor.is_empty() && ui.alt_down() {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            self.editor_data.selected = vec![tile_under_mouse];
            self.trigger(DragToLink);
            //
        } else if mouse.drag_started_by(Primary) && !under_cursor.is_empty() {
            //
            let tile_under_mouse = under_cursor.fetch_tile(&self.document_mosaic);
            if !self.editor_data.selected.contains(&tile_under_mouse) {
                self.editor_data.selected = vec![tile_under_mouse];
            }
            self.trigger(DragToMove);
            //
        } else if mouse.drag_started_by(egui::PointerButton::Primary) && under_cursor.is_empty() {
            //
            self.editor_data.selected = vec![];
            self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
            self.trigger(DragToSelect);
            //
        } else if mouse.drag_started_by(egui::PointerButton::Middle) {
            //
            self.trigger(DragToPan);
            //
        } else if mouse.drag_released_by(egui::PointerButton::Primary)
            || mouse.drag_released_by(egui::PointerButton::Middle)
        {
            //
            self.trigger(EndDrag);
            //
        } else if ui.mouse_secondary_down() {
            if !under_cursor.is_empty() && self.editor_data.selected.is_empty() {
                self.editor_data.selected = under_cursor.fetch_tiles(&self.document_mosaic);
            }
            self.response = Some(mouse.clone());
            self.trigger(ClickToContextMenu);
        }

        areas_to_remove.into_iter().for_each(|areas_vec: Vec<u64>| {
            for a in areas_vec {
                self.quadtree.delete_by_handle(a);
            }
        });
    }
}
