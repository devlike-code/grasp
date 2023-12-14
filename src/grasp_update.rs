use itertools::Itertools;
use mosaic::{
    capabilities::QueueCapability, internals::MosaicIO, iterators::tile_deletion::TileDeletion,
};

use crate::utilities::QuadTreeFetch;
use crate::GuiState;
use crate::{
    editor_state_machine::EditorState, grasp_common::GraspEditorWindow, math::rect::Rect2,
};

impl GraspEditorWindow {
    pub fn update(&mut self, s: &GuiState) {
        while let Some(request) = self.document_mosaic.dequeue(&self.tab_tile) {
            self.update_quadtree(None);
            request.iter().delete();
        }

        match &self.state {
            EditorState::Idle => {
                // if ui.input(|i| i.key_released(Key::Space)) {
                //     if let Some(queue) = self
                //         .document_mosaic
                //         .get_all()
                //         .include_component("NewTabRequestQueue")
                //         .get_targets()
                //         .next()
                //     {
                //         self.document_mosaic.enqueue(
                //             &queue,
                //             &take_objects(all_tiles()).to_tiles(&self.document_mosaic),
                //         );
                //     }
                // }
            }

            EditorState::Move => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
            }

            EditorState::Pan => {
                s.ui.set_mouse_cursor(Some(imgui::MouseCursor::Hand));
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
                        let rect = Rect2::from_two_pos(min, end_pos);

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

            EditorState::PropertyChanging => {}

            EditorState::Reposition => {}
            EditorState::ContextMenu => {}
        }
    }
}
