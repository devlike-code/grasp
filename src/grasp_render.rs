use std::ops::Add;

use egui::{Align2, Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use mosaic::{
    internals::{MosaicIO, TileFieldGetter},
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};

use crate::grasp_common::{get_pos_from_tile, GraspEditorTab};

impl GraspEditorTab {
    pub(crate) fn draw_arrow(
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
        println!("{:?}", dir);
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

    pub fn render(&mut self, ui: &mut Ui) {
        let painter = ui.painter();

        // Rendering
        for node in self
            .document_mosaic
            .get_all()
            .filter_objects()
            .include_component("Position")
        {
            // Draw node
            let pos = self.pos_with_pan(Pos2::new(node.get("x").as_f32(), node.get("y").as_f32()));
            painter.circle_filled(pos, 10.0, Color32::GRAY);

            // Maybe draw label
            if let Some(label) = node
                .into_iter()
                .get_descriptors()
                .include_component("Label")
                .next()
            {
                painter.text(
                    pos.add(Vec2::new(10.0, 10.0)),
                    Align2::LEFT_CENTER,
                    label.get("self").as_s32().to_string(),
                    FontId::default(),
                    Color32::GRAY,
                );
            }
        }

        for arrow in self.document_mosaic.get_all().filter_arrows() {
            let source_pos = self.pos_with_pan(
                get_pos_from_tile(&self.document_mosaic.get(arrow.source_id()).unwrap()).unwrap(),
            );

            let target_pos = self.pos_with_pan(
                get_pos_from_tile(&self.document_mosaic.get(arrow.target_id()).unwrap()).unwrap(),
            );

            Self::draw_arrow(
                painter,
                source_pos,
                target_pos - source_pos,
                Stroke::new(1.0, Color32::LIGHT_BLUE),
                10.0,
                10.0,
            );
        }

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

                ui.painter().rect(
                    rect,
                    Rounding::default(),
                    semi_transparent_light_blue,
                    stroke,
                );
            }
        }
    }
}
