use imgui::Key;
use itertools::Itertools;
use mosaic::internals::MosaicIO;

use crate::core::gui::imgui_keys::ExtraKeyEvents;
use crate::core::math::rect2::Rect2;
use crate::editor_state::windows::GraspEditorWindow;
use crate::editor_state_machine::EditorState;
use crate::utilities::QuadTreeFetch;
use crate::GuiState;

impl GraspEditorWindow {
    pub fn update(&mut self, s: &GuiState) {
        match &self.state {
            EditorState::Idle => {}
            EditorState::Move => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
            }

            EditorState::Pan => {
                s.ui.set_mouse_cursor(Some(imgui::MouseCursor::Hand));
                self.editor_data.pan += self.editor_data.cursor_delta;
            }

            EditorState::Link => {
                let quadtree = self.quadtree.lock().unwrap();
                let region = self.build_circle_area(
                    self.editor_data.cursor - self.editor_data.window_offset - self.editor_data.pan,
                    1,
                );
                let query = quadtree.query(region).collect_vec();
                if !query.is_empty() {
                    let tile_id = query.first().unwrap().value_ref();
                    if let Some(tile) = self.document_mosaic.get(*tile_id) {
                        if tile.is_object() || tile.is_arrow() {
                            self.editor_data.link_end = Some(tile);
                        }
                    }
                } else {
                    self.editor_data.link_end = None;
                }
            }

            EditorState::Rect => {
                if let Some(mut min) = self.editor_data.rect_start_pos {
                    min -= self.editor_data.window_offset;
                    if let Some(delta) = self.editor_data.rect_delta {
                        let end_pos = min + delta;
                        let rect = Rect2::from_two_pos(min, end_pos);
                        let quadtree = self.quadtree.lock().unwrap();
                        let region = self.build_rect_area(rect);
                        let query = quadtree.query(region).collect_vec();
                        if !query.is_empty() {
                            if s.ui.is_modkey_down(Key::LeftAlt)
                                || s.ui.is_modkey_down(Key::RightAlt)
                            {
                                self.editor_data.selected = query
                                    .fetch_tiles(&self.document_mosaic)
                                    .iter()
                                    .filter(|t| t.is_object() || t.is_arrow())
                                    .cloned()
                                    .collect_vec();
                            } else {
                                self.editor_data.selected = query
                                    .fetch_tiles(&self.document_mosaic)
                                    .iter()
                                    .filter(|t| t.is_object())
                                    .cloned()
                                    .collect_vec();
                            }
                        } else {
                            self.editor_data.selected = vec![];
                        }
                    }
                }
            }

            EditorState::PropertyChanging => {}
            EditorState::WindowResizing => {}

            EditorState::ContextMenu => {}

            EditorState::TransformerWorking => {}
        }
    }
}
