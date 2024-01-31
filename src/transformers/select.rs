use std::sync::Arc;

use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{par, pars, ComponentValuesBuilderSetter, MosaicIO, Tile, TileFieldEmptyQuery},
    iterators::tile_deletion::TileDeletion,
};

use crate::{
    core::math::Vec2,
    editor_state::{
        foundation::TransformerState, selection::SelectionTile, windows::GraspEditorWindow,
    },
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

        use random_color::color_dictionary::ColorDictionary;
        use random_color::{Luminosity, RandomColor};

        let color = RandomColor::new()
            .luminosity(Luminosity::Light)
            .alpha(0.5) // Optional
            .dictionary(ColorDictionary::new())
            .to_rgb_array();

        selection.add_component(
            "Color",
            pars()
                .set("r", color[0] as f32 / 255.0f32)
                .set("g", color[1] as f32 / 255.0f32)
                .set("b", color[2] as f32 / 255.0f32)
                .set("a", 0.5f32)
                .ok(),
        );

        deselect(window, ui, initial_state, tile);

        for selected in initial_state {
            selected.add_component("Selected", par(selection.id as u64));
        }

        let selection = SelectionTile::from_tile(selection);
    }

    TransformerState::Valid
}
