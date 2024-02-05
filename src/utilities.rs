use crate::core::math::vec2::Vec2;
use crate::core::math::Rect2;
use imgui::sys::ImVec4;
use itertools::Itertools;
use mosaic::internals::{EntityId, Mosaic, MosaicIO};
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{Tile, TileFieldEmptyQuery, TileFieldQuery, Value},
};
use quadtree_rs::entry::Entry;
use std::sync::Arc;

pub struct PosQuery<'a>(pub &'a Tile);
pub struct ColorQuery<'a>(pub &'a Tile);
pub struct RectQuery<'a>(pub &'a Tile);
pub struct OffsetQuery<'a>(pub &'a Tile);
pub struct SelfLoopQuery<'a>(pub &'a Tile);

impl<'a> TileFieldEmptyQuery for PosQuery<'a> {
    type Output = Vec2;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Position") {
            if let (Value::F32(x), Value::F32(y)) = pos_component.get_by(("x", "y")) {
                return Vec2::new(x, y);
            }
        }

        Default::default()
    }
}

impl<'a> TileFieldEmptyQuery for ColorQuery<'a> {
    type Output = ImVec4;
    fn query(&self) -> Self::Output {
        if let Some(color_component) = self.0.get_component("Color") {
            if let (Value::F32(r), Value::F32(g), Value::F32(b), Value::F32(a)) =
                color_component.get_by(("r", "g", "b", "a"))
            {
                return ImVec4::new(r, g, b, a);
            }
        }

        Default::default()
    }
}

impl<'a> TileFieldEmptyQuery for RectQuery<'a> {
    type Output = Rect2;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Rectangle") {
            if let (Value::F32(x), Value::F32(y), Value::F32(width), Value::F32(height)) =
                pos_component.get_by(("x", "y", "width", "height"))
            {
                return Rect2::from_pos_size(Vec2::new(x, y), Vec2::new(width, height));
            }
        }

        Default::default()
    }
}

impl<'a> TileFieldEmptyQuery for OffsetQuery<'a> {
    type Output = Vec2;
    fn query(&self) -> Self::Output {
        if let Some(offset_component) = self.0.get_component("Offset") {
            if let (Value::F32(x), Value::F32(y)) = offset_component.get_by(("x", "y")) {
                return Vec2::new(x, y);
            }
        }

        Default::default()
    }
}

impl<'a> TileFieldEmptyQuery for SelfLoopQuery<'a> {
    type Output = f32;
    fn query(&self) -> Self::Output {
        if let Some(offset_component) = self.0.get_component("SelfLoop") {
            return offset_component.get("self").as_f32();
        }

        Default::default()
    }
}

pub struct SelfText<'a>(pub &'a Tile, pub String);
impl<'a> TileFieldEmptyQuery for SelfText<'a> {
    type Output = String;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component(&self.1) {
            if let Value::S32(s) = pos_component.get("self") {
                return s.to_string();
            } else if let Value::STR(s) = pos_component.get("self") {
                return s;
            }
        }

        "".to_string()
    }
}

pub struct Process<'a>(pub &'a Tile);
impl<'a> TileFieldEmptyQuery for Process<'a> {
    type Output = String;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Process") {
            if let Value::S32(s) = pos_component.get("self") {
                return s.to_string();
            }
        }

        "".to_string()
    }
}

pub trait QuadTreeFetch {
    fn fetch_tiles(&self, mosaic: &Arc<Mosaic>) -> Vec<Tile>;
    fn fetch_tile(&self, mosaic: &Arc<Mosaic>) -> Tile;
}

impl QuadTreeFetch for Vec<&Entry<i32, EntityId>> {
    fn fetch_tiles(&self, mosaic: &Arc<Mosaic>) -> Vec<Tile> {
        self.iter()
            .flat_map(|next| mosaic.get(*next.value_ref()))
            .collect_vec()
    }

    fn fetch_tile(&self, mosaic: &Arc<Mosaic>) -> Tile {
        mosaic.get(*self.first().unwrap().value_ref()).unwrap()
    }
}

impl QuadTreeFetch for Vec<usize> {
    fn fetch_tiles(&self, mosaic: &Arc<Mosaic>) -> Vec<Tile> {
        self.iter().flat_map(|next| mosaic.get(*next)).collect_vec()
    }

    fn fetch_tile(&self, mosaic: &Arc<Mosaic>) -> Tile {
        mosaic.get(*self.first().unwrap()).unwrap()
    }
}
