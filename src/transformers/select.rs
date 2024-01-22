use mosaic::{capabilities::SelectionCapability, internals::Tile};

pub fn select(initial_state: &[Tile], window: &Tile) {
    if let Some(node) = initial_state.first() {
        node.mosaic.make_selection(initial_state);
    }
}
