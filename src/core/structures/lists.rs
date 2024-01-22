use std::{sync::Arc, vec::IntoIter};

use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{par, void, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

#[derive(Debug, Clone)]
pub struct ListTile(pub Tile);

impl AsRef<Tile> for ListTile {
    fn as_ref(&self) -> &Tile {
        &self.0
    }
}

impl From<ListTile> for Tile {
    fn from(val: ListTile) -> Self {
        val.0
    }
}

impl ListTile {
    pub fn from_tile(t: Tile) -> Option<ListTile> {
        if t.get_component("List").is_some() {
            Some(ListTile(t))
        } else {
            None
        }
    }

    pub fn add_back<T: AsRef<Tile>>(&self, element: T) {
        self.0.mosaic.add_back(self, element);
    }

    pub fn iter(&self) -> IntoIter<Tile> {
        self.0
            .iter()
            .get_extensions()
            .include_component("ListElement")
            .map(|le| self.0.mosaic.get(le.get("self").as_u64() as usize).unwrap())
            .collect_vec()
            .into_iter()
    }
}

pub trait ListCapability {
    fn make_list(&self) -> ListTile;
    fn add_back<T1: AsRef<Tile>, T2: AsRef<Tile>>(&self, list: T1, element: T2);
    fn list_count<T: AsRef<Tile>>(&self, list: T) -> usize;
}

impl ListCapability for Arc<Mosaic> {
    fn make_list(&self) -> ListTile {
        self.new_type("List: void;").unwrap();
        self.new_type("ListElement: u64;").unwrap();
        ListTile(self.new_object("List", void()))
    }

    fn add_back<T1: AsRef<Tile>, T2: AsRef<Tile>>(&self, list: T1, element: T2) {
        self.new_extension(
            list.as_ref(),
            "ListElement",
            par(element.as_ref().id as u64),
        );
    }

    fn list_count<T: AsRef<Tile>>(&self, list: T) -> usize {
        list.as_ref()
            .iter()
            .get_extensions()
            .include_component("ListElement")
            .count()
    }
}
