use std::vec::IntoIter;

use imgui::{sys::ImColor, DrawListMut, ImColor32};
use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{Tile, TileFieldEmptyQuery},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};

use crate::{
    core::{gui::windowing::gui_draw_image, math::Vec2},
    grasp_transitions::query_position_recursive,
    utilities::{ColorQuery, OffsetQuery, PosQuery},
    GuiState,
};

use super::{view::ComponentRenderer, windows::GraspEditorWindow};

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

    pub fn remove(&self, child: &Tile) {
        self.0
            .iter()
            .get_extensions()
            .include_component("Selection")
            .filter(|t| t.get("self").as_u64() as usize == child.id)
            .delete();
        println!("BEFORE: {:?}", child.get_components("Selected"));
        child.remove_components("Selected");
        println!("AFTER: {:?}", child.get_components("Selected"));
    }
}

pub fn pick_n_renderer(n: u32) -> ComponentRenderer {
    Box::new(
        move |s: &GuiState,
              window: &mut GraspEditorWindow,
              input: Tile,
              painter: &mut DrawListMut<'_>| {
            let pick = n.to_string();
            let sel = SelectionTile::from_tile(input.target());
            let col = ColorQuery(&input.target()).query();
            let mut min_x = 10000.0;
            let mut min_y = 10000.0;
            let mut max_x = -10000.0;
            let mut max_y = -10000.0;

            for part in sel.iter() {
                let pos = PosQuery(&part).query();
                if pos.x < min_x {
                    min_x = pos.x;
                }
                if pos.y < min_y {
                    min_y = pos.y;
                }
                if pos.x > max_x {
                    max_x = pos.x;
                }
                if pos.y > max_y {
                    max_y = pos.y;
                }
            }

            let min_xy = window.get_position_with_offset_and_pan(Vec2::new(min_x, min_y));
            let max_xy = window.get_position_with_offset_and_pan(Vec2::new(max_x, max_y));

            painter.add_rect_filled_multicolor(
                [min_xy.x - 60.0, min_xy.y - 60.0],
                [max_xy.x + 60.0, max_xy.y + 60.0],
                ImColor32::from_rgba_f32s(col.x, col.y, col.z, 0.2),
                ImColor32::from_rgba_f32s(col.x, col.y, col.z, 0.2),
                ImColor32::from_rgba_f32s(col.x, col.y, col.z, 0.2),
                ImColor32::from_rgba_f32s(col.x, col.y, col.z, 0.2),
            );

            painter.add_text(
                [min_xy.x - 40.0, min_xy.y - 40.0],
                ImColor32::WHITE,
                format!("Pick {}", pick).as_str(),
            );
        },
    )
}

pub fn selection_renderer(
    _s: &GuiState,
    window: &mut GraspEditorWindow,
    input: Tile,
    _painter: &mut DrawListMut<'_>,
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

        // painter.add_text(
        //     [pos.x - 25.0, pos.y - 25.0],
        //     ImColor32::from_rgba_f32s(color.x, color.y, color.z, color.w),
        //     format!("{}", selection.0.id),
        // );
    }
}
