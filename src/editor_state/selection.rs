use std::vec::IntoIter;

use imgui::{DrawListMut, ImColor32};
use mosaic::{
    capabilities::SelectionCapability,
    internals::{Tile, TileFieldEmptyQuery},
};

use crate::{core::math::Vec2, grasp_common::GraspEditorData, utilities::Pos, GuiState};

use super::windows::GraspEditorWindow;

pub struct SelectionTile(pub Tile);

impl AsRef<Tile> for SelectionTile {
    fn as_ref(&self) -> &Tile {
        &self.0
    }
}

impl SelectionTile {
    pub fn from_tile(input: Tile) -> SelectionTile {
        SelectionTile(input)
    }

    pub fn iter(&self) -> IntoIter<Tile> {
        self.0.mosaic.get_selection(&self.0)
    }
}

pub fn selection_renderer(
    s: &GuiState,
    window: &mut GraspEditorWindow,
    input: Tile,
    painter: &mut DrawListMut<'_>,
) {
    let selection = SelectionTile::from_tile(input);
    let mut min = Vec2::new(10000.0, 10000.0);
    let mut max = Vec2::new(-10000.0, -10000.0);

    for selected in selection.iter() {
        let pos = window.get_position_with_offset_and_pan(Pos(&selected).query());

        if pos.x < min.x {
            min.x = pos.x;
        }
        if pos.y < min.y {
            min.y = pos.y;
        }

        if pos.x > max.x {
            max.x = pos.x;
        }
        if pos.y > max.y {
            max.y = pos.y;
        }
    }

    painter.add_rect_filled_multicolor(
        [min.x - 50.0, min.y - 50.0],
        [max.x + 50.0, max.y + 50.0],
        ImColor32::from_rgba(77, 102, 128, 10),
        ImColor32::from_rgba(102, 77, 128, 10),
        ImColor32::from_rgba(77, 128, 102, 10),
        ImColor32::from_rgba(102, 128, 77, 10),
    );

    painter
        .add_rect(
            [min.x - 50.0, min.y - 50.0],
            [max.x + 50.0, max.y + 50.0],
            ImColor32::from_rgba(255, 255, 255, 25),
        )
        .build();

    for selected in selection.iter() {
        let pos = window.get_position_with_offset_and_pan(Pos(&selected).query());

        painter
            .add_circle([pos.x, pos.y], 20.0, ImColor32::WHITE)
            .build();

        painter.add_text(
            [pos.x + 8.0, pos.y + 8.0],
            ImColor32::WHITE,
            format!("{}", selection.0.id),
        );
    }
}
