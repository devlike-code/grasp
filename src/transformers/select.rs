use std::sync::Arc;

use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{par, pars, ComponentValuesBuilderSetter, Tile, TileFieldEmptyQuery},
};

use crate::{
    core::math::Vec2,
    editor_state::{helpers::QuadtreeUpdateCapability, selection::SelectionTile},
    utilities::Pos,
};

pub fn select(initial_state: &[Tile], window: &Tile) {
    if let Some(node) = initial_state.first() {
        let mosaic = Arc::clone(&node.mosaic);
        let selection = mosaic.make_selection(initial_state);
        for selected in initial_state {
            selected.add_component("Selected", par(selection.id as u64));
        }

        let selection = SelectionTile::from_tile(selection);
        let mut min = Vec2::new(10000.0, 10000.0);
        let mut max = Vec2::new(-10000.0, -10000.0);

        for selected in selection.iter() {
            let pos = Pos(&selected).query();

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

        selection.0.add_component(
            "Rectangle",
            pars()
                .set("x", min.x - 50.0)
                .set("y", min.y - 50.0)
                .set("width", max.x - min.x + 2.0 * 50.0)
                .set("height", max.y - min.y + 2.0 * 50.0)
                .ok(),
        );

        window.mosaic.request_quadtree_update();
    }
}
