use std::ops::Sub;

use egui::{Response, Sense, Ui, Vec2};
use itertools::Itertools;
use mosaic::{
    internals::{TileFieldGetter, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    editor_state_machine::{EditorStateTrigger, StateMachine},
    grasp_common::{GraspEditorTab, QuadTreeFetch, UiKeyDownExtract},
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
                self.editor_data.renaming = Some(tile.id);
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
        } else if mouse.drag_started_by(egui::PointerButton::Secondary) {
            //
            self.trigger(DragToPan);
            //
        } else if mouse.drag_released() {
            //
            self.trigger(EndDrag);
            //
        }
    }
}
