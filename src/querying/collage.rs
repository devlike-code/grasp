use itertools::Itertools;
use mosaic::{
    internals::{Mosaic, MosaicIO, Tile},
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Pick {
    Arrows,
    Descriptors,
    Extensions,
    Targets,
    Sources,
}

#[derive(Debug, Clone)]
pub enum Cut {
    Include(Vec<String>),
    Exclude(Vec<String>),
    Objects,
    Arrows,
    Descriptors,
    Extensions,
}

impl Cut {
    pub fn into_u8(&self) -> u8 {
        match self {
            Cut::Include(_) => 0,
            Cut::Exclude(_) => 1,
            Cut::Objects => 2,
            Cut::Arrows => 3,
            Cut::Descriptors => 4,
            Cut::Extensions => 5,
        }
    }
}

#[derive(Debug)]
pub enum Collage {
    Tiles,
    CombineQueries(Vec<Box<Collage>>),
    Pick(Pick, Box<Collage>),
    Cut(Cut, Box<Collage>),
}

pub trait MosaicCollage {
    fn apply_collage(&self, mq: &Collage, tiles: Option<Vec<Tile>>) -> std::vec::IntoIter<Tile>;
}

pub fn tiles() -> Box<Collage> {
    Box::new(Collage::Tiles)
}
pub fn arrows_from(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Pick(Pick::Arrows, mq))
}
pub fn descriptors_from(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Pick(Pick::Descriptors, mq))
}
pub fn extensions_from(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Pick(Pick::Extensions, mq))
}
pub fn targets_from(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Pick(Pick::Targets, mq))
}
pub fn sources_from(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Pick(Pick::Sources, mq))
}
pub fn take_components(comps: &[&str], mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Cut(
        Cut::Include(comps.iter().map(|s| s.to_string()).collect_vec()),
        mq,
    ))
}
pub fn leave_components(comps: &[&str], mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Cut(
        Cut::Exclude(comps.iter().map(|s| s.to_string()).collect_vec()),
        mq,
    ))
}
pub fn take_arrows(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Cut(Cut::Arrows, mq))
}
pub fn take_descriptors(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Cut(Cut::Descriptors, mq))
}
pub fn take_extensions(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Cut(Cut::Extensions, mq))
}
pub fn take_objects(mq: Box<Collage>) -> Box<Collage> {
    Box::new(Collage::Cut(Cut::Objects, mq))
}
pub fn gather(mqs: Vec<Box<Collage>>) -> Box<Collage> {
    Box::new(Collage::CombineQueries(mqs))
}

use std::sync::Arc;

use crate::querying::traversal::Traverse;

use super::traversal::Traversal;
impl MosaicCollage for Arc<Mosaic> {
    fn apply_collage(&self, mq: &Collage, tiles: Option<Vec<Tile>>) -> std::vec::IntoIter<Tile> {
        let traversal: Traversal = tiles.unwrap_or(self.get_all().collect_vec()).into();
        mq.query(Arc::clone(self), traversal)
    }
}
impl Collage {
    fn query(&self, mosaic: Arc<Mosaic>, traversal: Traversal) -> std::vec::IntoIter<Tile> {
        use Cut as F;
        use Pick as S;
        match self {
            Collage::Tiles => mosaic.traverse(traversal).get_all(),
            Collage::Pick(S::Arrows, b) => b.query(mosaic, traversal).get_arrows(),
            Collage::Pick(S::Descriptors, b) => b.query(mosaic, traversal).get_descriptors(),
            Collage::Pick(S::Extensions, b) => b.query(mosaic, traversal).get_extensions(),
            Collage::Pick(S::Targets, b) => b.query(mosaic, traversal).get_targets(),
            Collage::Pick(S::Sources, b) => b.query(mosaic, traversal).get_sources(),
            Collage::Cut(F::Include(components), b) => {
                b.query(mosaic, traversal).include_components(components)
            }
            Collage::Cut(F::Exclude(components), b) => {
                b.query(mosaic, traversal).exclude_components(components)
            }
            Collage::Cut(F::Arrows, b) => b.query(mosaic, traversal).filter_arrows(),
            Collage::Cut(F::Objects, b) => b.query(mosaic, traversal).filter_objects(),
            Collage::Cut(F::Descriptors, b) => b.query(mosaic, traversal).filter_descriptors(),
            Collage::Cut(F::Extensions, b) => b.query(mosaic, traversal).filter_extensions(),
            Collage::CombineQueries(bs) => bs
                .iter()
                .map(|b| b.query(Arc::clone(&mosaic), traversal.clone()))
                .fold(vec![].into_iter(), |all, next| {
                    all.chain(next).unique().collect_vec().into_iter()
                }),
        }
    }
}
