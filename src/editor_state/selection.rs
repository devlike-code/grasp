use std::vec::IntoIter;

use imgui::ImColor32;
use mosaic::{
    capabilities::SelectionCapability,
    internals::{Tile, TileFieldEmptyQuery},
};

use crate::{utilities::Pos, GuiState};

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

pub fn selection_renderer(s: &GuiState, window: &mut GraspEditorWindow, input: Tile) {
    let selection = SelectionTile::from_tile(input);
    println!("{:?}", selection.0);
    for selected in selection.iter() {
        let painter = s.ui.get_window_draw_list();
        let pos = Pos(&selected).query();
        painter
            .add_circle([pos.x, pos.y], 20.0, ImColor32::WHITE)
            .build();
    }
}
