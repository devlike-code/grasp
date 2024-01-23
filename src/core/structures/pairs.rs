use std::sync::Arc;

use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{
        pars, ComponentValuesBuilderSetter, EntityId, Mosaic, MosaicIO, MosaicTypelevelCRUD, Tile,
    },
};

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

    pub fn get_first(&self) -> Tile {
        self.0.mosaic.get_first(&self.0)
    }

    pub fn get_second(&self) -> Tile {
        self.0.mosaic.get_second(&self.0)
    }
}

pub trait PairCapability<Id> {
    fn make_pair(&self, fst: &Id, snd: &Id) -> PairTile;
    fn get_first(&self, pair: &Tile) -> Id;
    fn get_second(&self, pair: &Tile) -> Id;
}

impl PairCapability<EntityId> for Arc<Mosaic> {
    fn make_pair(&self, fst: &EntityId, snd: &EntityId) -> PairTile {
        self.new_type("Pair: { first: u64, second: u64 };").unwrap();
        PairTile(
            self.new_object(
                "Pair",
                pars()
                    .set("first", *fst as u64)
                    .set("second", *snd as u64)
                    .ok(),
            ),
        )
    }

    fn get_first(&self, pair: &Tile) -> EntityId {
        pair.get("first").as_u64() as usize
    }

    fn get_second(&self, pair: &Tile) -> EntityId {
        pair.get("second").as_u64() as usize
    }
}

impl PairCapability<Tile> for Arc<Mosaic> {
    fn make_pair(&self, fst: &Tile, snd: &Tile) -> PairTile {
        Self::make_pair(self, &fst.id, &snd.id)
    }

    fn get_first(&self, pair: &Tile) -> Tile {
        self.get(pair.get("first").as_u64() as usize).unwrap()
    }

    fn get_second(&self, pair: &Tile) -> Tile {
        self.get(pair.get("second").as_u64() as usize).unwrap()
    }
}
