use std::sync::Arc;

use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{par, MosaicIO, Tile, TileFieldEmptyQuery},
};

use crate::{core::math::Vec2, editor_state::selection::SelectionTile, utilities::PosQuery};

pub fn find_selection_owner(selected_tile: &Tile) -> Option<SelectionTile> {
    selected_tile
        .get_component("Selected")
        .and_then(|t| selected_tile.mosaic.get(t.get("self").as_u64() as usize))
        .map(SelectionTile::from_tile)
}

pub fn deselect(initial_state: &[Tile], _window: &Tile) {
    for selected in initial_state {
        if let Some(previous_selection) = find_selection_owner(selected) {
            previous_selection.remove(selected);
        }
    }
}

pub fn select(initial_state: &[Tile], window: &Tile) {
    if let Some(node) = initial_state.first() {
        let mosaic = Arc::clone(&node.mosaic);
        let selection = mosaic.make_selection(initial_state);

        deselect(initial_state, window);

        for selected in initial_state {
            selected.add_component("Selected", par(selection.id as u64));
        }

        let selection = SelectionTile::from_tile(selection);
        let mut min = Vec2::new(10000.0, 10000.0);
        let mut max = Vec2::new(-10000.0, -10000.0);

        for selected in selection.iter() {
            let pos = PosQuery(&selected).query();

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
    }
}
