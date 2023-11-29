use egui::{CursorIcon, Rect, Ui};
use itertools::Itertools;
use mosaic::internals::MosaicIO;

use crate::{
    editor_state_machine::EditorState,
    grasp_common::{GraspEditorTab, QuadTreeFetch},
};

impl GraspEditorTab {
    pub fn update(&mut self, ui: &mut Ui) {
        match &self.state {
            EditorState::Idle => {}

            EditorState::Move => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
            }

            EditorState::Pan => {
                ui.ctx().set_cursor_icon(CursorIcon::Move);
                self.editor_data.pan += self.editor_data.cursor_delta;
            }

            EditorState::Link => {
                let region = self.build_circle_area(self.editor_data.cursor, 1);
                let query = self.quadtree.query(region).collect_vec();
                if !query.is_empty() {
                    let tile_id = query.first().unwrap().value_ref();
                    self.editor_data.link_end = self.document_mosaic.get(*tile_id);
                } else {
                    self.editor_data.link_end = None;
                }
            }

            EditorState::Rect => {
                if let Some(min) = self.editor_data.rect_start_pos {
                    if let Some(delta) = self.editor_data.rect_delta {
                        let end_pos = min + delta;
                        let rect = Rect::from_two_pos(min, end_pos);

                        let region = self.build_rect_area(rect);
                        let query = self.quadtree.query(region).collect_vec();
                        if !query.is_empty() {
                            self.editor_data.selected = query.fetch_tiles(&self.document_mosaic);
                        } else {
                            self.editor_data.selected = vec![];
                        }
                    }
                }
            }
        }
    }
}
