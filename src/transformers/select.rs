use std::sync::Arc;

use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{par, pars, ComponentValuesBuilderSetter, MosaicIO, Tile, TileFieldEmptyQuery},
    iterators::tile_deletion::TileDeletion,
};

use crate::{
    core::{math::Vec2, structures::enqueue},
    editor_state::{
        foundation::TransformerState, selection::SelectionTile, windows::GraspEditorWindow,
    },
    grasp_queues::WindowTileDeleteReactionRequestQueue,
    utilities::PosQuery,
    GuiState,
};

pub fn find_selection_owner(selected_tile: &Tile) -> Option<SelectionTile> {
    selected_tile
        .get_component("Selected")
        .and_then(|t| selected_tile.mosaic.get(t.get("self").as_u64() as usize))
        .map(SelectionTile::from_tile)
}

pub fn deselect(
    _window: &GraspEditorWindow,
    _ui: &GuiState,
    initial_state: &[Tile],
    _tile: &Tile,
) -> TransformerState {
    for selected in initial_state {
        if let Some(previous_selection) = find_selection_owner(selected) {
            previous_selection.remove(selected);

            if previous_selection.iter().len() == 0 {
                previous_selection.0.iter().delete();
            }
        }
    }

    TransformerState::Valid
}

pub fn on_selected_delete(window: &mut GraspEditorWindow, comp: String, selected: &Tile) {
    assert_eq!(&comp, "Selected");

    if let Some(previous_selection) = find_selection_owner(selected) {
        previous_selection.remove(selected);

        if previous_selection.iter().len() == 0 {
            window.delete_tiles(&[previous_selection.0]);
        }
    }
}

pub fn select(
    window: &GraspEditorWindow,
    ui: &GuiState,
    initial_state: &[Tile],
    tile: &Tile,
) -> TransformerState {
    if let Some(node) = initial_state.first() {
        let mosaic = Arc::clone(&node.mosaic);
        let selection = mosaic.make_selection(initial_state);

        deselect(window, ui, initial_state, tile);

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

    TransformerState::Valid
}
