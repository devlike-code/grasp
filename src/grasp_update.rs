use egui::{Color32, CursorIcon, Rect, Stroke, Ui};
use itertools::Itertools;
use mosaic::internals::MosaicIO;

use crate::{
    editor_state_machine::EditorState,
    grasp_common::{get_pos_from_tile, GraspEditorTab},
};

impl GraspEditorTab {
    pub fn update(&mut self, ui: &mut Ui) {
        match &self.state {
            EditorState::Idle => {}

            EditorState::Move => {
                self.update_position_for_selected(self.editor_data.cursor);
            }

            EditorState::Pan => {
                ui.ctx().set_cursor_icon(CursorIcon::Move);
                self.editor_data.pan += self.editor_data.cursor_delta;
            }

            EditorState::Link => {
                if let Some(start_pos) = self.editor_data.link_start_pos {
                    let mut end_pos = self.editor_data.cursor;
                    let mut end_offset = 0.0;
                    if let Some(end) = &self.editor_data.link_end {
                        end_pos = get_pos_from_tile(end).unwrap();
                        end_offset = 10.0;
                    }

                    Self::draw_arrow(
                        ui.painter(),
                        start_pos,
                        end_pos - start_pos,
                        Stroke::new(2.0, Color32::LIGHT_GREEN),
                        10.0,
                        end_offset,
                    )
                }

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
                            self.editor_data.selected = query
                                .into_iter()
                                .flat_map(|e| self.document_mosaic.get(*e.value_ref()))
                                .collect_vec();
                        } else {
                            self.editor_data.selected = vec![];
                        }
                    }
                }
            }
        }
    }
}
