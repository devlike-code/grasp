use std::{sync::Arc, vec::IntoIter};

use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{par, void, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

#[derive(Debug, Clone)]
pub struct ListTile(pub Tile);

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

    pub fn add_back(&self, element: &Tile) {
        self.0.mosaic.add_back(&self.0, element);
    }

    pub fn iter(&self) -> IntoIter<Tile> {
        self.0
            .iter()
            .get_extensions()
            .include_component("ListElement")
    }
}

pub trait ListCapability {
    fn make_list(&self) -> ListTile;
    fn add_back(&self, list: &Tile, element: &Tile);
    fn list_count(&self, list: &Tile) -> usize;
}

impl ListCapability for Arc<Mosaic> {
    fn make_list(&self) -> ListTile {
        self.new_type("List: void;").unwrap();
        self.new_type("ListElement: void;").unwrap();
        ListTile(self.new_object("List", void()))
    }

    fn add_back(&self, list: &Tile, element: &Tile) {
        self.new_extension(list, "ListElement", par(element.id as u64));
    }

    fn list_count(&self, list: &Tile) -> usize {
        list.iter()
            .get_extensions()
            .include_component("ListElement")
            .count()
    }
}
