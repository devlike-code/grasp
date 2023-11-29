use std::ops::Sub;

use egui::{Sense, Ui, Vec2};
use itertools::Itertools;
use mosaic::internals::MosaicIO;

use crate::{
    editor_state_machine::{EditorStateTrigger, StateMachine},
    grasp_common::GraspEditorTab,
};

impl GraspEditorTab {
    pub fn sense(&mut self, ui: &mut Ui) {
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

        let result = self
            .quadtree
            .query(self.build_circle_area(self.editor_data.cursor, 1))
            .collect_vec();

        if resp.double_clicked() && result.is_empty() {
            self.trigger(EditorStateTrigger::DblClickToCreate);
        } else if resp.drag_started_by(egui::PointerButton::Primary) && !result.is_empty() {
            let is_alt_down = {
                let mut alt_down = false;
                ui.input(|input_state| {
                    alt_down = input_state.modifiers.alt;
                });
                alt_down
            };

            if is_alt_down {
                self.editor_data.selected = vec![self
                    .document_mosaic
                    .get(*result.first().unwrap().value_ref())
                    .unwrap()];
                self.trigger(EditorStateTrigger::DragToLink);
            } else {
                self.editor_data.selected = result
                    .into_iter()
                    .flat_map(|next| self.document_mosaic.get(*next.value_ref()))
                    .collect_vec();
                self.trigger(EditorStateTrigger::DragToMove);
            }
            println!("---------------DRAAAG");
        } else if resp.drag_started_by(egui::PointerButton::Primary) && result.is_empty() {
            self.editor_data.selected = vec![];

            self.editor_data.rect_start_pos = Some(self.editor_data.cursor);

            self.trigger(EditorStateTrigger::DragToSelect);
        } else if resp.drag_started_by(egui::PointerButton::Secondary) {
            self.trigger(EditorStateTrigger::DragToPan);
        } else if resp.drag_released() {
            self.trigger(EditorStateTrigger::EndDrag);
        }
    }
}
