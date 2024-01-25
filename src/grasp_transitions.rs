use itertools::Itertools;
use log::warn;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{MosaicIO, Tile, TileFieldEmptyQuery, TileFieldQuery, TileFieldSetter, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::{
        gui::calc_text_size,
        math::{vec2::Vec2, Rect2},
    },
    editor_state::windows::GraspEditorWindow,
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    transformers::TransformerUtilities,
    utilities::{OffsetQuery, PosQuery, RectQuery},
};

impl StateMachine for GraspEditorWindow {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        let previous = if self.editor_mosaic.is_transformer_pending() {
            Some(EditorState::TransformerWorking)
        } else {
            Some(EditorState::Idle)
        };

        let result = match (from, trigger) {
            (EditorState::ContextMenu, EditorStateTrigger::ExitContextMenu) => {
                if self.under_cursor().is_empty() {
                    self.editor_data.selected.clear();
                }

                previous
            }
            (EditorState::ContextMenu, EditorStateTrigger::TransformerSelected) => {
                Some(EditorState::TransformerWorking)
            }
            (EditorState::ContextMenu, _) => None,
            (_, EditorStateTrigger::ClickToContextMenu) => Some(EditorState::ContextMenu),
            (EditorState::Idle, EditorStateTrigger::TransformerSelected) => {
                Some(EditorState::TransformerWorking)
            }
            (EditorState::TransformerWorking, EditorStateTrigger::TransformerDone) => previous,
            (EditorState::TransformerWorking, EditorStateTrigger::TransformerCancelled) => previous,

            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(
                    self.editor_data.cursor - self.editor_data.window_offset - self.editor_data.pan,
                );
                self.changed = true;
                //all windows need to update their quadtrees
                self.request_quadtree_update();
                previous
            }
            (_, EditorStateTrigger::DblClickToRename) => Some(EditorState::PropertyChanging),
            (_, EditorStateTrigger::MouseDownOverNode) => None,
            (_, EditorStateTrigger::ClickToSelect) => previous,

            (EditorState::Pan, EditorStateTrigger::ClickToDeselect) => {
                self.request_quadtree_update();
                previous
            }
            (_, EditorStateTrigger::ClickToDeselect) => {
                self.editor_data.selected.clear();
                previous
            }
            (_, EditorStateTrigger::DragToPan) => {
                self.editor_data.previous_pan = self.editor_data.pan;
                Some(EditorState::Pan)
            }
            (_, EditorStateTrigger::DragToWindowResize) => Some(EditorState::WindowResizing),
            (_, EditorStateTrigger::DragToLink) => {
                let position_from_tile =
                    query_position_recursive(self.editor_data.selected.first().unwrap());
                self.editor_data.link_start_pos = Some(
                    position_from_tile + self.editor_data.window_offset + self.editor_data.pan,
                );

                Some(EditorState::Link)
            }
            (_, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => {
                self.editor_data.rect_delta = Some(Default::default());
                self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
                Some(EditorState::Rect)
            }
            (EditorState::Pan, EditorStateTrigger::EndDrag) => {
                self.request_quadtree_update();
                previous
            }
            (EditorState::WindowResizing, EditorStateTrigger::EndDrag) => previous,
            (EditorState::Link, EditorStateTrigger::EndDrag) => {
                if let Some(tile) = self.editor_data.link_end.take() {
                    let start = self.editor_data.selected.first().unwrap().clone();
                    let mut src_pos = Vec2::default();
                    let mut tgt_pos = Vec2::default();
                    if let (
                        (Value::F32(s_x), Value::F32(s_y)),
                        (Value::F32(t_x), Value::F32(t_y)),
                    ) = (
                        start.get_component("Position").unwrap().get_by(("x", "y")),
                        tile.get_component("Position").unwrap().get_by(("x", "y")),
                    ) {
                        src_pos = Vec2::new(s_x, s_y);
                        tgt_pos = Vec2::new(t_x, t_y);
                    }

                    let mid_pos = src_pos.lerp(tgt_pos, 0.5);

                    self.create_new_arrow(&start, &tile, mid_pos);
                    self.changed = true;
                }

                self.editor_data.link_start_pos = None;
                self.editor_data.link_end = None;

                // all windows need to update their quadtrees
                self.request_quadtree_update();
                previous
            }
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
                self.changed = true;
                self.request_quadtree_update();
                previous
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => {
                self.editor_data.rect_start_pos = None;
                self.editor_data.rect_delta = None;
                previous
            }
            (EditorState::PropertyChanging, _) => {
                self.editor_data.tile_changing = None;
                self.editor_data.field_changing = None;
                self.editor_data.previous_text.clear();
                self.editor_data.text.clear();
                self.changed = true;
                previous
            }

            (s, t) => {
                warn!("TRANSITION NOT DEALT WITH: {:?} {:?}!", s, t);
                None
            }
        };

        if result.is_some() {
            println!("from {:?} trigger {:?}: Executed", from, trigger);
        } else {
            println!("from {:?} trigger {:?}: Not Executed", from, trigger);
        }
        result
    }

    fn get_current_state(&self) -> Self::State {
        self.state
    }

    fn move_to(&mut self, next: Self::State) {
        self.state = next;
    }
}

impl GraspEditorWindow {
    pub fn update_selected_positions_by(&mut self, dp: Vec2) {
        for tile in &mut self.editor_data.selected {
            let component_name = if tile.is_object() {
                "Position"
            } else {
                "Offset"
            };

            if let Some(mut selected_pos_component) = tile.get_component(component_name) {
                if let (Value::F32(x), Value::F32(y)) = selected_pos_component.get_by(("x", "y")) {
                    selected_pos_component.set("x", x + dp.x);
                    selected_pos_component.set("y", y + dp.y);
                }
            }
        }
    }

    pub fn update_quadtree(&mut self, _selection: Option<Vec<Tile>>) {
        fn find_arrow_pos(arrow: &Tile) -> Vec2 {
            let start_pos = query_position_recursive(&arrow.source());
            let end_pos = query_position_recursive(&arrow.target());
            let mid = start_pos.lerp(end_pos, 0.5);
            //Arrow offset
            let offset = OffsetQuery(arrow).query();
            mid + offset
        }

        let rects = self
            .document_mosaic
            .get_all()
            .include_component("Rectangle")
            .get_targets()
            .collect_vec();

        let selected = self
            .document_mosaic
            .get_all()
            .include_component("Position")
            .get_targets()
            .collect_vec();

        self.quadtree.lock().unwrap().reset();

        for tile in &selected {
            let mut selected_pos = PosQuery(tile).query();

            if tile.is_arrow() {
                selected_pos = find_arrow_pos(tile);
            }

            if let Some(label) = tile.get_component("Label") {
                let size = calc_text_size(label.get("self").as_s32().to_string());

                if let Some(offset) = label.get_component("Offset") {
                    let off = OffsetQuery(&offset).query();
                    let label_region = self.build_label_area(Rect2 {
                        x: selected_pos.x + off.x,
                        y: selected_pos.y + off.y,
                        width: size[0],
                        height: size[1],
                    });

                    let mut quadtree = self.quadtree.lock().unwrap();
                    quadtree.insert(label_region, label.id);
                }
            }

            if tile.is_object() {
                let mut quadtree = self.quadtree.lock().unwrap();
                let region = self.build_circle_area(selected_pos, 12);
                if let Some(area_id) = quadtree.insert(region, tile.id) {
                    self.object_to_area.lock().unwrap().insert(tile.id, area_id);
                }
            } else if tile.is_arrow() {
                let region = self.build_circle_area(selected_pos, 12);
                let mut quadtree = self.quadtree.lock().unwrap();
                if let Some(area_id) = quadtree.insert(region, tile.id) {
                    self.object_to_area.lock().unwrap().insert(tile.id, area_id);
                }
            }
        }

        for tile in &rects {
            let rect = RectQuery(tile).query();
            let region = self.build_label_area(rect);
            let mut quadtree = self.quadtree.lock().unwrap();
            if let Some(area_id) = quadtree.insert(region, tile.id) {
                self.object_to_area.lock().unwrap().insert(tile.id, area_id);
            }
        }
    }
}

pub fn query_position_recursive(tile: &Tile) -> Vec2 {
    let pos = PosQuery(tile).query();
    let offset = OffsetQuery(tile).query();
    if tile.is_arrow() {
        let src = query_position_recursive(&tile.source());
        let tgt = query_position_recursive(&tile.target());
        src.lerp(tgt, 0.5) + offset
    } else {
        pos
    }
}
