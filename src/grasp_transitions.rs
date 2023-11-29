use egui::{Pos2, Vec2};
use mosaic::internals::TileFieldSetter;

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{get_pos_from_tile, GraspEditorTab},
};

impl StateMachine for GraspEditorTab {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        println!("{:?} {:?}", from, trigger);
        match (from, trigger) {
            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(self.editor_data.cursor);
                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::MouseDownOverNode) => None,
            (_, EditorStateTrigger::ClickToSelect) => None,
            (_, EditorStateTrigger::ClickToDeselect) => {
                self.editor_data.selected.clear();
                Some(EditorState::Idle)
            }
            (EditorState::Idle, EditorStateTrigger::DragToPan) => {
                self.editor_data.previous_pan = self.editor_data.pan;
                Some(EditorState::Pan)
            }
            (EditorState::Idle, EditorStateTrigger::DragToLink) => {
                self.editor_data.link_start_pos =
                    get_pos_from_tile(self.editor_data.selected.first().unwrap());
                Some(EditorState::Link)
            }
            (EditorState::Idle, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => {
                self.editor_data.rect_delta = Some(Vec2::ZERO);
                self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
                Some(EditorState::Rect)
            }
            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Link, EditorStateTrigger::EndDrag) => {
                if let Some(tile) = self.editor_data.link_end.take() {
                    let start = self.editor_data.selected.first().unwrap().clone();
                    self.create_new_arrow(&start, &tile);
                }
                self.editor_data.selected.clear();
                self.editor_data.link_start_pos = None;
                self.editor_data.link_end = None;
                Some(EditorState::Idle)
            }
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_position_for_selected(self.editor_data.cursor);
                self.update_quadtree_for_selected();
                Some(EditorState::Idle)
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => {
                self.editor_data.rect_start_pos = None;
                self.editor_data.rect_delta = None;
                Some(EditorState::Idle)
            }
            _ => None,
        }
    }

    fn get_current_state(&self) -> Self::State {
        self.state
    }

    fn move_to(&mut self, next: Self::State) {
        self.state = next;
    }
}

impl GraspEditorTab {
    pub fn update_position_for_selected(&mut self, pos: Pos2) {
        for tile in &mut self.editor_data.selected {
            tile.set("x", pos.x);
            tile.set("y", pos.y);
        }
    }

    pub fn update_quadtree_for_selected(&mut self) {
        for tile in &self.editor_data.selected {
            if let Some(area_id) = self.node_area.get(&tile.id) {
                self.quadtree.delete_by_handle(*area_id);

                let region = self.build_circle_area(self.editor_data.cursor, 10);
                if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                    self.node_area.insert(tile.id, area_id);
                }
            }
        }
    }
}
