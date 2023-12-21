use std::ops::Add;

use crate::core::gui::windowing::get_texture;
use crate::core::math::bezier::gui_draw_bezier_arrow;
use crate::editor_state_machine::EditorState;
use crate::grasp_editor_window::GraspEditorWindow;

use crate::core::math::vec2::Vec2;
use crate::utilities::{Label, Pos};
use crate::GuiState;
use imgui::sys::{ImVec2, ImVec4};
use imgui::ImColor32;
use mosaic::internals::MosaicIO;
use mosaic::internals::TileFieldEmptyQuery;
use mosaic::iterators::component_selectors::ComponentSelectors;
use mosaic::iterators::tile_getters::TileGetters;

pub trait GraspRenderer {
    fn draw(&self, window: &GraspEditorWindow, s: &GuiState);
    fn get_position_with_pan(&self, window: &GraspEditorWindow, position: Vec2) -> Vec2;
}

pub struct DefaultGraspRenderer;

impl GraspRenderer for DefaultGraspRenderer {
    fn draw(&self, window: &GraspEditorWindow, s: &GuiState) {
        let mut painter = s.ui.get_window_draw_list();

        let tiles = window
            .document_mosaic
            .get_all()
            .include_component("Position")
            .get_targets();

        if tiles.len() > 0 {
            for tile in tiles {
                if tile.is_object() {
                    let pos = self.get_position_with_pan(window, Pos(&tile).query());
                    let label = Label(&tile).query();

                    painter
                        .add_circle([pos.x, pos.y], 10.0, ImColor32::from_rgb(255, 0, 0))
                        .build();
                    unsafe {
                        imgui::sys::igSetCursorPos(ImVec2::new(
                            pos.x - window.rect.x - 10.0,
                            pos.y - window.rect.y - 10.0,
                        ));
                        imgui::sys::igImage(
                            get_texture("dot") as *mut _,
                            ImVec2::new(20.0, 20.0),
                            ImVec2::new(0.0, 0.0),
                            ImVec2::new(1.0, 1.0),
                            ImVec4::new(1.0, 1.0, 1.0, 1.0),
                            ImVec4::new(0.0, 0.0, 0.0, 0.0),
                        );
                    }
                    painter.add_text([pos.x + 10.0, pos.y], ImColor32::WHITE, label);
                }
            }
        }

        let arrows = window.document_mosaic.get_all().include_component("Arrow");

        for arrow in arrows {
            let p1 = self.get_position_with_pan(window, Pos(&arrow.source()).query());
            let p2 = self.get_position_with_pan(window, Pos(&arrow.target()).query());
            let a: [f32; 2] = p1.into();
            let b: [f32; 2] = p2.into();
            gui_draw_bezier_arrow(
                &mut painter,
                [p1, p1.lerp(p2, 0.5), p2],
                2.0,
                ImColor32::WHITE,
            );
            /*
            draw_list: &mut DrawListMut<'a>,
            points: [Vec2; 4],
            thickness: f32,
            color: ImColor32,
            start_arrow: BezierArrowHead,
            end_arrow: BezierArrowHead,
            */
            painter.add_line(a, b, ImColor32::WHITE).build();
        }

        match window.state {
            EditorState::Link => {
                let a: [f32; 2] = window.editor_data.link_start_pos.unwrap().into();
                let pos: [f32; 2] = if let Some(b) = window.editor_data.link_end.as_ref() {
                    self.get_position_with_pan(window, Pos(b).query()).into()
                } else {
                    s.ui.io().mouse_pos
                };

                painter.add_line(a, pos, ImColor32::WHITE).build();
            }
            EditorState::Rect => {
                let a: [f32; 2] = {
                    let position = window.editor_data.rect_start_pos.unwrap();
                    self.get_position_with_pan(window, position).into()
                };

                let b: [f32; 2] = {
                    let position = window.editor_data.rect_start_pos.unwrap()
                        + window.editor_data.rect_delta.unwrap();
                    self.get_position_with_pan(window, position).into()
                };

                painter.add_rect_filled_multicolor(
                    a,
                    b,
                    ImColor32::from_rgba(77, 102, 128, 30),
                    ImColor32::from_rgba(102, 77, 128, 30),
                    ImColor32::from_rgba(77, 128, 102, 30),
                    ImColor32::from_rgba(102, 128, 77, 30),
                );
                painter
                    .add_rect(a, b, ImColor32::from_rgba(255, 255, 255, 40))
                    .build();
            }
            _ => {}
        }
    }

    fn get_position_with_pan(&self, window: &GraspEditorWindow, position: Vec2) -> Vec2 {
        position.add(window.editor_data.pan)
    }
}
