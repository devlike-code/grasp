use std::vec::IntoIter;

use imgui::{DrawListMut, ImColor32};
use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{Tile, TileFieldEmptyQuery},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};

use crate::{
    core::{
        gui::windowing::gui_draw_image,
        math::{Rect2, Vec2},
    },
    grasp_transitions::query_position_recursive,
    utilities::{ColorQuery, OffsetQuery, PosQuery},
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

    pub fn remove(&self, child: &Tile) {
        self.0
            .iter()
            .get_extensions()
            .include_component("Selection")
            .filter(|t| t.get("self").as_u64() as usize == child.id)
            .delete();
        child.remove_components("Selected");
    }

    pub fn delete(&self) {
        let _ = self.iter().map(|t| t.remove_components("Selected"));
        self.0.iter().delete();
    }
}

pub fn selection_renderer(
    _s: &GuiState,
    window: &mut GraspEditorWindow,
    input: Tile,
    painter: &mut DrawListMut<'_>,
) {
    let selection = SelectionTile::from_tile(input);
    let color = ColorQuery(&selection.0).query();
    for selected in selection.iter() {
        let mut pos = window.get_position_with_offset_and_pan(PosQuery(&selected).query());
        if selected.is_arrow() {
            let p1 = window
                .get_position_with_offset_and_pan(query_position_recursive(&selected.source()));
            let p2 = window
                .get_position_with_offset_and_pan(query_position_recursive(&selected.target()));
            let offset = OffsetQuery(&selected).query();
            let mid = p1.lerp(p2, 0.5) + offset;
            pos = mid;
        }

        gui_draw_image(
            if selected.is_arrow() {
                "selection-arrow"
            } else {
                "selection"
            },
            [30.0, 30.0],
            [pos.x - window.rect.x, pos.y - window.rect.y],
            0.0,
            1.0,
            Some(color),
        );

        painter.add_text(
            [pos.x - 25.0, pos.y - 25.0],
            ImColor32::from_rgba_f32s(color.x, color.y, color.z, color.w),
            format!("{}", selection.0.id),
        );
    }
}
