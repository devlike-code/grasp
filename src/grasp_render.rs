use std::sync::Arc;

use crate::grasp_editor_window::GraspEditorWindow;

use crate::core::math::vec2::Vec2;
use crate::utilities::{Label, Pos};
use crate::GuiState;
use imgui::ImColor32;
use log::info;
use mosaic::internals::{Mosaic, TileFieldEmptyQuery, TileFieldQuery};
use mosaic::internals::{MosaicIO, Tile};
use mosaic::iterators::component_selectors::ComponentSelectors;
use mosaic::iterators::tile_filters::TileFilters;
use mosaic::iterators::tile_getters::TileGetters;
use mosaic::{capabilities::ArchetypeSubject, internals::MosaicCollage};

pub trait GraspRenderer {
    fn draw(&self, mosaic: &Arc<Mosaic>, s: &GuiState);
}

pub struct DefaultGraspRenderer;

impl GraspRenderer for DefaultGraspRenderer {
    fn draw(&self, mosaic: &Arc<Mosaic>, s: &GuiState) {
        let painter = s.ui.get_window_draw_list();

        let tiles = mosaic.get_all().include_component("Position").get_targets();
        if tiles.len() > 0 {
            for tile in tiles {
                if tile.is_object() {
                    let pos = Pos(&tile).query();
                    let label = Label(&tile).query();

                    painter
                        .add_circle([pos.x, pos.y], 10.0, ImColor32::from_rgb(255, 0, 0))
                        .build();
                }
            }
        }
    }
}
