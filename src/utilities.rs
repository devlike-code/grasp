use crate::core::math::vec2::Vec2;
use itertools::Itertools;
use mosaic::internals::{EntityId, Mosaic, MosaicIO};
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{Tile, TileFieldEmptyQuery, TileFieldQuery, Value},
};
use quadtree_rs::entry::Entry;
use std::sync::Arc;

pub struct Pos<'a>(pub &'a Tile);

impl<'a> TileFieldEmptyQuery for Pos<'a> {
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
pub struct Label<'a>(pub &'a Tile);
impl<'a> TileFieldEmptyQuery for Label<'a> {
    type Output = String;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Label") {
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
