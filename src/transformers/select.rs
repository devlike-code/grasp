use std::sync::Arc;

use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{par, Tile},
};

pub fn select(initial_state: &[Tile], _window: &Tile) {
    if let Some(node) = initial_state.first() {
        let mosaic = Arc::clone(&node.mosaic);
        let selection = mosaic.make_selection(initial_state);
        for selected in initial_state {
            selected.add_component("Selected", par(selection.id as u64));
        }
    }
}
