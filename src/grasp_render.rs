use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rangef, Rect, Rounding, Stroke, TextEdit, Ui, Vec2,
};
use mosaic::{
    internals::{MosaicIO, Tile, TileFieldGetter},
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};
use std::ops::Add;

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{get_pos_from_tile, GraspEditorTab},
};

impl GraspEditorTab {
    fn internal_draw_arrow(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        vec: Vec2,
        stroke: Stroke,
        start_offset: f32,
        end_offset: f32,
    ) {
        let rot = egui::emath::Rot2::from_angle(std::f32::consts::TAU / 15.0);
        let tip_length = 15.0;
        let dir = vec.normalized();
        let a_start: Pos2 = origin + dir * start_offset;
        let tip = a_start + vec - dir * (start_offset + end_offset);
        let middle = a_start.lerp(tip, 0.5);

        let shape = egui::epaint::QuadraticBezierShape {
            points: [a_start, middle, tip],
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: Stroke {
                width: 2.0,
                color: Color32::LIGHT_BLUE,
            },
        };
        painter.add(shape);
        painter.line_segment([tip, tip - tip_length * (rot * dir)], stroke);
        painter.line_segment([tip, tip - tip_length * (rot.inverse() * dir)], stroke);
    }

    fn draw_arrow(&mut self, painter: &Painter, arrow: &Tile) {
        let source_pos = self.pos_with_pan(
            get_pos_from_tile(&self.document_mosaic.get(arrow.source_id()).unwrap()).unwrap(),
        );

        let target_pos = self.pos_with_pan(
            get_pos_from_tile(&self.document_mosaic.get(arrow.target_id()).unwrap()).unwrap(),
        );

        self.internal_draw_arrow(
            painter,
            source_pos,
            target_pos - source_pos,
            Stroke::new(1.0, Color32::LIGHT_BLUE),
            10.0,
            10.0,
        );
    }

    fn draw_node(&mut self, ui: &mut Ui, node: &Tile) {
        let painter = ui.painter();

        // Draw node
        let pos = self.pos_with_pan(Pos2::new(node.get("x").as_f32(), node.get("y").as_f32()));
        painter.circle_filled(pos, 10.0, Color32::GRAY);

        if let Some(label) = node
            .iter()
            .get_descriptors()
            .include_component("Label")
            .next()
        {
            let floating_pos = pos.add(Vec2::new(10.0, 10.0));

            if self.state == EditorState::Rename && self.editor_data.renaming == Some(node.id) {
                let text_edit = TextEdit::singleline(&mut self.editor_data.text)
                    .char_limit(30)
                    .cursor_at_end(true);
                let text_edit_response = ui.put(
                    Rect::from_two_pos(
                        floating_pos.add(Vec2::new(0.0, -5.0)),
                        floating_pos.add(Vec2::new(60.0, 20.0)),
                    ),
                    text_edit,
                );

                text_edit_response.request_focus();

                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.trigger(EditorStateTrigger::EndDrag);
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editor_data.text = self.editor_data.previous_text.clone();
                    self.trigger(EditorStateTrigger::EndDrag);
                }
            } else {
                painter.text(
                    floating_pos,
                    Align2::LEFT_CENTER,
                    label.get("self").as_s32().to_string(),
                    FontId::default(),
                    Color32::GRAY,
                );
            }
        }
    }

    fn draw_link(&mut self, painter: &Painter) {
        if self.state != EditorState::Link {
            return;
        }

        if let Some(start_pos) = self.editor_data.link_start_pos {
            let mut end_pos = self.editor_data.cursor;
            let mut end_offset = 0.0;
            if let Some(end) = &self.editor_data.link_end {
                end_pos = get_pos_from_tile(end).unwrap();
                end_offset = 10.0;
            }

            self.internal_draw_arrow(
                painter,
                start_pos,
                end_pos - start_pos,
                Stroke::new(2.0, Color32::WHITE),
                10.0,
                end_offset,
            )
        }
    }

    fn draw_selected(&mut self, painter: &Painter) {
        for selected in &self.editor_data.selected {
            let stroke = Stroke {
                width: 0.5,
                color: Color32::RED,
            };
            let selected_pos = self.pos_with_pan(Pos2::new(
                selected.get("x").as_f32(),
                selected.get("y").as_f32(),
            ));

            painter.circle(selected_pos, 11.0, Color32::RED, stroke);
        }
    }

    fn draw_rect_select(&mut self, painter: &Painter) {
        if self.state != EditorState::Rect {
            return;
        }
        if let Some(min) = self.editor_data.rect_start_pos {
            if let Some(delta) = self.editor_data.rect_delta {
                let min = self.pos_with_pan(min);
                let end_pos = min + delta;
                let semi_transparent_light_blue = Color32::from_rgba_unmultiplied(255, 120, 255, 2);
                let rect = Rect::from_two_pos(min, end_pos);

                let stroke = Stroke {
                    width: 0.5,
                    color: Color32::LIGHT_BLUE,
                };

                painter.rect(
                    rect,
                    Rounding::default(),
                    semi_transparent_light_blue,
                    stroke,
                );
            }
        }
    }

    pub fn draw_ruler(&mut self, ui: &mut Ui) {
        let painter = ui.painter();
        let max_rect = ui.max_rect();
        let pan = self.editor_data.pan;
        let min_x = max_rect.min.x;
        let max_x = max_rect.max.x;
        let min_y = max_rect.min.y;
        let max_y = max_rect.max.y;

        // Draw horizontal ruler
        {
            painter.rect_filled(
                Rect {
                    min: (min_x, min_y).into(),
                    max: (max_x, min_y + 20.0).into(),
                },
                Rounding::default(),
                Color32::DARK_GRAY,
            );

            let step = 50;
            let low_bound = (min_x - pan.x) as i32 / step;
            let upper_bound = (max_x - pan.x) as i32 / step;
            let range_y = Rangef::new(min_y + 10.0 as f32, 20.0);
            for i in low_bound..(upper_bound + 1) {
                painter.vline(
                    (i * step) as f32 + pan.x,
                    range_y,
                    Stroke::new(2.0, Color32::WHITE),
                );
                painter.text(
                    ((i * step) as f32 + pan.x, min_y + 10.0).into(),
                    Align2::CENTER_CENTER,
                    format!("{}", i * step),
                    FontId::default(),
                    Color32::WHITE,
                );
            }
        }

        // Draw vertical ruler
        {
            painter.rect_filled(
                Rect {
                    min: (min_x, min_y).into(),
                    max: (min_x + 20.0, max_y).into(),
                },
                Rounding::default(),
                Color32::DARK_GRAY,
            );

            let step = 50;
            let low_bound = (min_y - pan.y) as i32 / step;
            let upper_bound = (max_y - pan.y) as i32 / step;
            let range_x = Rangef::new(min_x + 20.0, 20.0);
            for i in low_bound..(upper_bound + 1) {
                painter.hline(
                    range_x,
                    (i * step) as f32 + pan.y,
                    Stroke::new(2.0, Color32::WHITE),
                );
                painter.text(
                    (min_x + 15.0, (i * step) as f32 + pan.y).into(),
                    Align2::CENTER_CENTER,
                    format!("{}", i * step),
                    FontId::default(),
                    Color32::WHITE,
                );
            }
        }

        self.draw_pointer_position(painter, min_x, min_y);
    }

    pub fn draw_pointer_position(&mut self, painter: &Painter, min_x: f32, min_y: f32) {
        painter.rect_filled(
            Rect {
                min: (min_x, min_y).into(),
                max: (min_x + 35.0, min_y + 20.0).into(),
            },
            Rounding::default(),
            Color32::DARK_GRAY,
        );

        painter.text(
            (min_x, min_y).into(),
            Align2::LEFT_TOP,
            format!("({:.2},", self.editor_data.cursor.x),
            FontId::proportional(7.0),
            Color32::WHITE,
        );

        painter.text(
            (min_x, min_y + 10.0).into(),
            Align2::LEFT_TOP,
            format!("{:.2})", self.editor_data.cursor.y),
            FontId::proportional(7.0),
            Color32::WHITE,
        );
    }

    pub fn render(&mut self, ui: &mut Ui) {
        for node in self
            .document_mosaic
            .get_all()
            .filter_objects()
            .include_component("Position")
        {
            self.draw_node(ui, &node);
        }

        let painter = ui.painter();

        for arrow in self.document_mosaic.get_all().filter_arrows() {
            self.draw_arrow(painter, &arrow);
        }

        self.draw_link(painter);
        self.draw_selected(painter);
        self.draw_rect_select(painter);
        if self.ruler_visible { 
            self.draw_ruler(ui);
        }
    }
}
