use std::{sync::Arc, vec::IntoIter};

use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{par, void, Mosaic, MosaicCRUD, MosaicIO, Tile},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};

use crate::editor_state::windows::GraspEditorWindow;

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
            .include_component("ListMember")
            .filter_map(|le| self.0.mosaic.get(le.get("self").as_u64() as usize))
            .collect_vec()
            .into_iter()
    }
}

pub trait ListCapability {
    fn make_list(&self) -> ListTile;
    fn add_back<T1: AsRef<Tile>, T2: AsRef<Tile>>(&self, list: T1, element: T2);
}

impl ListCapability for Arc<Mosaic> {
    fn make_list(&self) -> ListTile {
        ListTile(self.new_object("List", void()))
    }

    fn add_back<T1: AsRef<Tile>, T2: AsRef<Tile>>(&self, list: T1, element: T2) {
        self.new_descriptor(
            element.as_ref(),
            "ListElement",
            par(list.as_ref().id as u64),
        );

        self.new_extension(list.as_ref(), "ListMember", par(element.as_ref().id as u64));
    }
}

pub fn on_list_element_deleted(window: &mut GraspEditorWindow, comp: String, le: &Tile) {
    assert_eq!(&comp, "ListElement");
    let main = le.target();

    if let Some(list) = window
        .document_mosaic
        .get(le.get("self").as_u64() as usize)
        .and_then(ListTile::from_tile)
    {
        list.0
            .iter()
            .get_extensions()
            .include_component("ListMember")
            .filter(|e| e.get("self").as_u64() == main.id as u64)
            .delete();

        println!(
            "LIST HAS {} MEMBERS NOW: {:?}",
            list.iter().len(),
            list.iter()
        );
        if list.iter().len() == 0 {
            window.delete_tiles(&[list.0]);
        }
    }
}
