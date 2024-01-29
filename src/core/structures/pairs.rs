use std::sync::Arc;

use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{
        par, pars, ComponentValuesBuilderSetter, EntityId, Mosaic, MosaicCRUD, MosaicIO,
        MosaicTypelevelCRUD, Tile,
    },
    iterators::tile_deletion::TileDeletion,
};

use crate::editor_state::windows::GraspEditorWindow;

#[derive(Debug, Clone)]
pub struct PairTile(pub Tile);

impl AsRef<Tile> for PairTile {
    fn as_ref(&self) -> &Tile {
        &self.0
    }
}

impl From<PairTile> for Tile {
    fn from(val: PairTile) -> Self {
        val.0
    }
}

impl PairTile {
    pub fn from_tile(t: Tile) -> Option<PairTile> {
        if t.get_component("Pair").is_some() {
            Some(PairTile(t))
        } else {
            None
        }
    }

    pub fn get_first(&self) -> Option<Tile> {
        self.0.mosaic.get_first(&self.0)
    }

    pub fn get_second(&self) -> Option<Tile> {
        self.0.mosaic.get_second(&self.0)
    }
}

pub trait PairCapability<Id> {
    fn make_pair(&self, fst: &Id, snd: &Id) -> PairTile;
    fn get_first(&self, pair: &Tile) -> Option<Id>;
    fn get_second(&self, pair: &Tile) -> Option<Id>;
}

impl PairCapability<EntityId> for Arc<Mosaic> {
    fn make_pair(&self, fst: &EntityId, snd: &EntityId) -> PairTile {
        let pair = PairTile(
            self.new_object(
                "Pair",
                pars()
                    .set("first", *fst as u64)
                    .set("second", *snd as u64)
                    .ok(),
            ),
        );

        self.new_descriptor(fst, "PairElement", par(pair.0.id as u64));
        self.new_descriptor(snd, "PairElement", par(pair.0.id as u64));
        pair
    }

    fn get_first(&self, pair: &Tile) -> Option<EntityId> {
        let f = pair.get("first").as_u64() as usize;
        if pair.mosaic.is_tile_valid(&f) {
            Some(f)
        } else {
            None
        }
    }

    fn get_second(&self, pair: &Tile) -> Option<EntityId> {
        let f = pair.get("second").as_u64() as usize;
        if pair.mosaic.is_tile_valid(&f) {
            Some(f)
        } else {
            None
        }
    }
}

impl PairCapability<Tile> for Arc<Mosaic> {
    fn make_pair(&self, fst: &Tile, snd: &Tile) -> PairTile {
        Self::make_pair(self, &fst.id, &snd.id)
    }

    fn get_first(&self, pair: &Tile) -> Option<Tile> {
        self.get(pair.get("first").as_u64() as usize)
    }

    fn get_second(&self, pair: &Tile) -> Option<Tile> {
        self.get(pair.get("second").as_u64() as usize)
    }
}

pub fn on_pair_element_deleted(window: &mut GraspEditorWindow, comp: String, pm: &Tile) {
    assert_eq!(&comp, "PairElement");
    println!("{:?}", pm);
    if let Some(pair) = window
        .document_mosaic
        .get(pm.get("self").as_u64() as usize)
        .and_then(PairTile::from_tile)
    {
        let _ = pair.get_first().map(|t| {
            t.get_components("PairElement")
                .iter()
                .filter(|t| t.get("self").as_u64() as usize == pair.0.id)
                .for_each(|t| t.iter().delete());
        });
        let _ = pair.get_second().map(|t| {
            t.get_components("PairElement")
                .iter()
                .filter(|t| t.get("self").as_u64() as usize == pair.0.id)
                .for_each(|t| t.iter().delete());
        });

        window.delete_tiles(&[pair.0]);
    } else {
        println!("{:?} DOESN'T EXIST", pm);
    }
}
