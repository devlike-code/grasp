use std::sync::Arc;

use itertools::Itertools;
use log::warn;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{
        void, Mosaic, MosaicCRUD, MosaicIO, Tile, TileFieldEmptyQuery, TileFieldQuery,
        TileFieldSetter, Value,
    },
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    core::{
        math::{vec2::Vec2, Rect2},
        queues::enqueue,
    },
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_editor_window::GraspEditorWindow,
    grasp_editor_window_list::get_pos_from_tile,
    grasp_queues::QuadtreeUpdateRequestQueue,
    utilities::Pos,
};

pub trait QuadtreeUpdateCapability {
    fn request_quadtree_update(&self);
}

impl QuadtreeUpdateCapability for Arc<Mosaic> {
    fn request_quadtree_update(&self) {
        enqueue(
            QuadtreeUpdateRequestQueue,
            self.new_object("QuadtreeUpdateRequest", void()),
        );
    }
}

impl StateMachine for GraspEditorWindow {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        println!("from {:?} trigger {:?}", from, trigger);
        match (from, trigger) {
            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(self.editor_data.cursor);
                self.document_mosaic.request_quadtree_update();

                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::DblClickToRename) => Some(EditorState::PropertyChanging),
            (_, EditorStateTrigger::ClickToReposition) => Some(EditorState::Reposition),

            (_, EditorStateTrigger::MouseDownOverNode) => None,
            (_, EditorStateTrigger::ClickToSelect) => Some(EditorState::Idle),
            (_, EditorStateTrigger::ClickToDeselect) => {
                self.editor_data.selected.clear();
                Some(EditorState::Idle)
            }
            (_, EditorStateTrigger::DragToPan) => {
                self.editor_data.previous_pan = self.editor_data.pan;
                Some(EditorState::Pan)
            }
            (_, EditorStateTrigger::DragToLink) => {
                if let Some(position_from_tile) =
                    get_pos_from_tile(self.editor_data.selected.first().unwrap())
                {
                    self.editor_data.link_start_pos = Some(self.pos_with_pan(position_from_tile));
                }
                Some(EditorState::Link)
            }
            (_, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (_, EditorStateTrigger::ClickToContextMenu) => Some(EditorState::ContextMenu),
            (EditorState::ContextMenu, _) => {
                //self.response = None;
                self.on_transition(EditorState::Idle, trigger)
            }
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => {
                self.editor_data.rect_delta = Some(Default::default());
                self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
                Some(EditorState::Rect)
            }

            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
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
                    // Calculate the angle and radius for the new control point
                    let _angle: f32 = 45.0; // Adjust the angle as needed
                    let _radius: f32 = 50.0; // Adjust the radius as needed

                    let bez = (1..=9)
                        .map(|i| src_pos.lerp(tgt_pos, i as f32 / 10.0))
                        .map(|p| {
                            Rect2::from_two_pos(p - Vec2::new(5.0, 5.0), p + Vec2::new(5.0, 5.0))
                        });

                    self.create_new_arrow(&start, &tile, mid_pos, bez.collect_vec());
                }
                self.editor_data.selected.clear();
                self.editor_data.link_start_pos = None;
                self.editor_data.link_end = None;
                self.document_mosaic.request_quadtree_update();
                Some(EditorState::Idle)
            }
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
                self.document_mosaic.request_quadtree_update();
                Some(EditorState::Idle)
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => {
                self.editor_data.rect_start_pos = None;
                self.editor_data.rect_delta = None;
                Some(EditorState::Idle)
            }

            (EditorState::PropertyChanging, _) => {
                self.editor_data.tile_changing = None;
                self.editor_data.field_changing = None;
                self.editor_data.previous_text.clear();
                self.editor_data.text.clear();
                Some(EditorState::Idle)
            }

            (EditorState::Reposition, _) => {
                if let Some(tile_id) = self.editor_data.repositioning {
                    if self.document_mosaic.is_tile_valid(&tile_id) {
                        if let Some(mut pos) = self
                            .document_mosaic
                            .get(tile_id)
                            .unwrap()
                            .get_component("Position")
                        {
                            pos.set(
                                "x",
                                self.editor_data.x_pos.parse::<f32>().unwrap_or_default(),
                            );
                            pos.set(
                                "y",
                                self.editor_data.y_pos.parse::<f32>().unwrap_or_default(),
                            );

                            self.update_quadtree(Some(vec![pos]));
                        }
                    }
                }

                self.editor_data.tile_changing = None;
                self.editor_data.previous_text.clear();
                self.editor_data.text.clear();
                Some(EditorState::Idle)
            }

            (s, t) => {
                warn!("TRANSITION NOT DEALT WITH: {:?} {:?}!", s, t);
                None
            }
        }
    }

    fn get_current_state(&self) -> Self::State {
        self.state
    }

    fn move_to(&mut self, next: Self::State) {
        self.state = next;
    }
}

impl GraspEditorWindow {
    // pub fn generate_rects_for_bezier(qb: QuadraticBezierShape) -> Vec<Rect2> {
    //     //  let samples = qb.flatten(Some(0.1));

    //     let mut samples = vec![];

    //     // samples.push(qb.sample(0.0));
    //     // for i in 1..21 {
    //     //     samples.push(qb.sample(i as f32 / 20.0));
    //     // }

    //     let mut rects = vec![];

    //     #[allow(clippy::comparison_chain)]
    //     if samples.len() > 2 {
    //         for i in (0..samples.len()).step_by(2) {
    //             if i + 2 < samples.len() {
    //                 let rect = Rect2::from_two_pos(samples[i], samples[i + 2]);
    //                 rects.push(rect);
    //             }
    //             if i + 3 < samples.len() {
    //                 let rect = Rect2::from_two_pos(samples[i + 1], samples[i + 3]);
    //                 rects.push(rect);
    //             }
    //         }
    //     } else if samples.len() == 2 {
    //         let rect = Rect2::from_two_pos(samples[0], samples[1]);
    //         rects.push(rect);
    //     }
    //     rects
    // }

    pub fn update_selected_positions_by(&mut self, dp: Vec2) {
        for tile in &mut self.editor_data.selected {
            let mut selected_pos_component = tile.get_component("Position").unwrap();
            if let (Value::F32(x), Value::F32(y)) = selected_pos_component.get_by(("x", "y")) {
                selected_pos_component.set("x", x + dp.x);
                selected_pos_component.set("y", y + dp.y);
            }
        }
    }

    //TO-DO! --- Decide if we need selection and implement it, instead of re-creating everything every time it's called
    //       --- If and when its impelmented pay atention to those not connected but stil updated because of connecting arrows
    pub fn update_quadtree(&mut self, _selection: Option<Vec<Tile>>) {
        let selected = self
            .document_mosaic
            .get_all()
            .include_component("Position")
            .get_targets()
            .collect_vec();

        self.quadtree.lock().unwrap().reset();

        for tile in &selected {
            let selected_pos = Pos(tile).query();

            if tile.is_object() {
                let mut quadtree = self.quadtree.lock().unwrap();
                let region = self.build_circle_area(selected_pos, 12);
                if let Some(area_id) = quadtree.insert(region, tile.id) {
                    self.object_to_area
                        .lock()
                        .unwrap()
                        .insert(tile.id, vec![area_id]);
                }
            } else if tile.is_arrow() {
                let start_pos = Pos(&tile.source()).query();
                let end_pos = Pos(&tile.target()).query();

                let bez = (1..=9)
                    .map(|i| start_pos.lerp(end_pos, i as f32 / 10.0))
                    .map(|p| Rect2::from_two_pos(p - Vec2::new(5.0, 5.0), p + Vec2::new(5.0, 5.0)));

                for r in bez {
                    let region = self.build_rect_area(r);
                    let mut quadtree = self.quadtree.lock().unwrap();
                    if let Some(area_id) = quadtree.insert(region, tile.id) {
                        let mut object_to_area = self.object_to_area.lock().unwrap();
                        if let Some(areas_vec) = object_to_area.get_mut(&tile.id) {
                            areas_vec.push(area_id);
                        } else {
                            object_to_area.insert(tile.id, vec![area_id]);
                        }
                    }
                }
            }
        }
    }
}
