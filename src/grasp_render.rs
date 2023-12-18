use std::sync::Arc;

use crate::editor_state_machine::EditorState;
use crate::grasp_editor_window::GraspEditorWindow;

use crate::core::math::vec2::Vec2;
use crate::utilities::{Label, Pos};
use crate::GuiState;
use imgui::ImColor32;
use log::{debug, info};
use mosaic::internals::{Mosaic, TileFieldEmptyQuery, TileFieldQuery};
use mosaic::internals::{MosaicIO, Tile};
use mosaic::iterators::component_selectors::ComponentSelectors;
use mosaic::iterators::tile_filters::TileFilters;
use mosaic::iterators::tile_getters::TileGetters;
use mosaic::{capabilities::ArchetypeSubject, internals::MosaicCollage};

pub trait GraspRenderer {
    fn draw(&self, window: &GraspEditorWindow, s: &GuiState);
}

pub struct DefaultGraspRenderer;

impl GraspRenderer for DefaultGraspRenderer {
    fn draw(&self, window: &GraspEditorWindow, s: &GuiState) {
        let painter = s.ui.get_window_draw_list();

        let tiles = window
            .document_mosaic
            .get_all()
            .include_component("Position")
            .get_targets();

        if tiles.len() > 0 {
            for tile in tiles {
                if tile.is_object() {
                    let pos = Pos(&tile).query();
                    let label = Label(&tile).query();

                    painter
                        .add_circle([pos.x, pos.y], 10.0, ImColor32::from_rgb(255, 0, 0))
                        .build();

                    painter.add_text([pos.x + 10.0, pos.y], ImColor32::WHITE, label);
                }
            }
        }

        let arrows = window.document_mosaic.get_all().include_component("Arrow");

        for arrow in arrows {
            let a: [f32; 2] = Pos(&arrow.source()).query().into();
            let b: [f32; 2] = Pos(&arrow.target()).query().into();
            painter.add_line(a, b, ImColor32::WHITE).build();
        }

        match window.state {
            EditorState::Link => {
                let a: [f32; 2] = window.editor_data.link_start_pos.unwrap().into();
                let pos: [f32; 2] = if let Some(b) = window.editor_data.link_end.as_ref() {
                    Pos(b).query().into()
                } else {
                    s.ui.io().mouse_pos
                };

                painter.add_line(a, pos, ImColor32::WHITE).build();
            }
            EditorState::Rect => {
                let a: [f32; 2] = window.editor_data.rect_start_pos.unwrap().into();
                let b: [f32; 2] = (window.editor_data.rect_start_pos.unwrap()
                    + window.editor_data.rect_delta.unwrap())
                .into();
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
}
