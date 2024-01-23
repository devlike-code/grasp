use std::vec::IntoIter;

use imgui::{DrawListMut, ImColor32};
use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{Tile, TileFieldEmptyQuery},
};

use crate::{
    core::math::{Rect2, Vec2},
    utilities::Pos,
    GuiState,
};

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

    pub fn rectangle(&self) -> Rect2 {
        if let Some(rect) = self.0.get_component("Rectangle") {
            Rect2 {
                x: rect.get("x").as_f32(),
                y: rect.get("y").as_f32(),
                width: rect.get("width").as_f32(),
                height: rect.get("height").as_f32(),
            }
        } else {
            Rect2::default()
        }
    }
}

pub fn selection_renderer(
    _s: &GuiState,
    window: &mut GraspEditorWindow,
    input: Tile,
    painter: &mut DrawListMut<'_>,
) {
    let selection = SelectionTile::from_tile(input);
    let rect = selection.rectangle();
    let min = window.get_position_with_offset_and_pan(rect.min());
    let max = window.get_position_with_offset_and_pan(rect.max());
    painter.add_rect_filled_multicolor(
        [min.x, min.y],
        [max.x, max.y],
        ImColor32::from_rgba(77, 102, 128, 10),
        ImColor32::from_rgba(102, 77, 128, 10),
        ImColor32::from_rgba(77, 128, 102, 10),
        ImColor32::from_rgba(102, 128, 77, 10),
    );

    painter
        .add_rect(
            [min.x, min.y],
            [max.x, max.y],
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
