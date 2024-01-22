#[derive(Debug, Default, PartialEq, Clone)]
pub enum TraversalDirection {
    #[default]
    Forward,
    Backward,
    Both,
}

use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    vec::IntoIter,
};

use itertools::Itertools;
use mosaic::{
    internals::{
        sparse_matrix::{BidirectionalMatrix, Matrix},
        Mosaic, MosaicIO, Tile, TileGetById,
    },
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};

#[derive(Clone)]
pub enum Traversal<'a> {
    Exclude {
        components: &'a [String],
    },
    Include {
        components: &'a [String],
    },
    Limited {
        tiles: Vec<Tile>,
        include_arrows: bool,
    },
    Default,
}

impl From<Vec<Tile>> for Traversal<'_> {
    fn from(value: Vec<Tile>) -> Self {
        Traversal::Limited {
            tiles: value,
            include_arrows: true,
        }
    }
}

impl From<IntoIter<Tile>> for Traversal<'_> {
    fn from(value: IntoIter<Tile>) -> Self {
        Traversal::Limited {
            tiles: value.collect_vec(),
            include_arrows: true,
        }
    }
}

pub struct TraversalOperator<'a> {
    pub(crate) mosaic: Arc<Mosaic>,
    pub(crate) traversal: Traversal<'a>,
}

impl TraversalOperator<'_> {
    pub fn get_all(&self) -> IntoIter<Tile> {
        self.filter_traversal(self.mosaic.get_all()).into_iter()
    }

    fn filter_traversal<I: Iterator<Item = Tile>>(&self, iter: I) -> Vec<Tile> {
        match &self.traversal {
            Traversal::Exclude { components } => iter.exclude_components(components).collect_vec(),
            Traversal::Include { components } => iter.include_components(components).collect_vec(),
            Traversal::Limited {
                tiles,
                include_arrows: true,
            } => iter
                .filter(|t| {
                    let mosaic = Arc::clone(&t.mosaic);
                    let s = mosaic.get(t.source_id()).unwrap();
                    let t = mosaic.get(t.target_id()).unwrap();
                    tiles.contains(&s) && tiles.contains(&t)
                })
                .collect_vec(),
            Traversal::Limited {
                tiles,
                include_arrows: false,
            } => iter.filter(|t| tiles.contains(t)).collect_vec(),
            Traversal::Default => iter.collect_vec(),
        }
    }

    pub fn out_degree(&self, tile: &Tile) -> usize {
        self.filter_traversal(tile.clone().into_iter().get_arrows_from())
            .len()
    }

    pub fn get_objects(&self) -> IntoIter<Tile> {
        match &self.traversal {
            Traversal::Limited { tiles, .. } => tiles.clone().into_iter().filter_objects(),
            _ => self.mosaic.get_all().filter_objects(),
        }
    }

    pub fn get_arrows_into(&self, tile: &Tile) -> IntoIter<Tile> {
        self.filter_traversal(tile.clone().into_iter().get_arrows_into())
            .into_iter()
    }

    pub fn get_arrows_from(&self, tile: &Tile) -> IntoIter<Tile> {
        self.filter_traversal(tile.clone().into_iter().get_arrows_from())
            .into_iter()
    }

    pub fn get_self_loops(&self, tile: &Tile) -> IntoIter<Tile> {
        self.filter_traversal(
            tile.clone()
                .into_iter()
                .get_arrows_from()
                .filter(|a| a.is_loop())
                .unique(),
        )
        .into_iter()
    }

    pub fn in_degree(&self, tile: &Tile) -> usize {
        self.filter_traversal(tile.clone().into_iter().get_arrows_into())
            .len()
    }

    pub fn get_forward_neighbors(&self, tile: &Tile) -> IntoIter<Tile> {
        self.filter_traversal(
            tile.clone()
                .into_iter()
                .get_arrows_from()
                .filter(|a| !a.is_loop()),
        )
        .into_iter()
        .get_targets()
    }

    pub fn get_backward_neighbors(&self, tile: &Tile) -> IntoIter<Tile> {
        self.filter_traversal(
            tile.clone()
                .into_iter()
                .get_arrows_into()
                .filter(|a| !a.is_loop()),
        )
        .into_iter()
        .get_sources()
    }

    pub fn get_neighbors(&self, tile: &Tile) -> IntoIter<Tile> {
        let mut result = self.get_backward_neighbors(tile).collect_vec();
        result.extend(self.get_forward_neighbors(tile));
        result.into_iter()
    }

    pub fn as_matrix(&self) -> BidirectionalMatrix {
        let mut matrix = BidirectionalMatrix::default();
        for node in self.get_objects() {
            matrix.add_node(node.id);
        }

        for node in self.get_objects() {
            for arrow in self.get_arrows_from(&node) {
                matrix.add_edge(arrow.id, arrow.source_id(), arrow.target_id());
            }
        }

        matrix
    }

    pub fn depth_first_search(&self, from: &Tile, direction: TraversalDirection) -> Vec<Vec<Tile>> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((vec![], HashSet::new(), from.id));

        while let Some((mut trek, visited, current_id)) = queue.pop_front() {
            let current = self.mosaic.get(current_id).unwrap();
            trek.push(current.id);

            let neighbors = match direction {
                TraversalDirection::Forward => self.get_forward_neighbors(&current).collect_vec(),
                TraversalDirection::Backward => self.get_backward_neighbors(&current).collect_vec(),
                TraversalDirection::Both => self.get_neighbors(&current).collect_vec(),
            };

            if !neighbors.is_empty() {
                let mut recursive = false;
                for neighbor in neighbors {
                    if !visited.contains(&neighbor.id) {
                        recursive = true;
                        let mut next_visited = visited.clone();
                        next_visited.insert(current.id);
                        queue.push_back((trek.clone(), next_visited, neighbor.id));
                    }
                }

                if !recursive {
                    result.push(trek.clone());
                }
            } else {
                result.push(trek.clone());
            }
        }

        result
            .into_iter()
            .map(|path| self.mosaic.get_tiles(path).collect_vec())
            .collect_vec()
    }

    pub fn get_forward_paths(&self, from: &Tile) -> Vec<Vec<Tile>> {
        self.depth_first_search(from, TraversalDirection::Forward)
    }

    pub fn get_forward_path_between(&self, src: &Tile, tgt: &Tile) -> Option<Vec<Tile>> {
        let reach = self.get_forward_paths(src);
        let path = reach
            .into_iter()
            .flatten()
            .filter(|t| t == tgt)
            .collect_vec();

        if !path.is_empty() {
            Some(path)
        } else {
            None
        }
    }

    pub fn forward_path_exists_between(&self, src: &Tile, tgt: &Tile) -> bool {
        self.get_forward_path_between(src, tgt).is_some()
    }

    pub fn get_backward_paths(&self, from: &Tile) -> Vec<Vec<Tile>> {
        self.depth_first_search(from, TraversalDirection::Backward)
    }

    pub fn get_backward_path_between(&self, src: &Tile, tgt: &Tile) -> Option<Vec<Tile>> {
        let reach = self.get_backward_paths(src);
        let path = reach
            .into_iter()
            .flatten()
            .filter(|t| t == tgt)
            .collect_vec();

        if !path.is_empty() {
            Some(path)
        } else {
            None
        }
    }

    pub fn backward_path_exists_between(&self, src: &Tile, tgt: &Tile) -> bool {
        self.get_backward_path_between(src, tgt).is_some()
    }

    pub fn get_paths(&self, from: &Tile) -> Vec<Vec<Tile>> {
        self.depth_first_search(from, TraversalDirection::Both)
    }

    pub fn get_path_between(&self, src: &Tile, tgt: &Tile) -> Option<Vec<Tile>> {
        let reach = self.get_paths(src);
        let path = reach
            .into_iter()
            .flatten()
            .filter(|t| t == tgt)
            .collect_vec();

        if !path.is_empty() {
            Some(path)
        } else {
            None
        }
    }

    pub fn path_exists_between(&self, src: &Tile, tgt: &Tile) -> bool {
        self.get_path_between(src, tgt).is_some()
    }
}

pub trait Traverse<'a> {
    fn traverse(&self, traversal: Traversal<'a>) -> TraversalOperator<'a>;
}

impl<'a> Traverse<'a> for Arc<Mosaic> {
    fn traverse(&self, traversal: Traversal<'a>) -> TraversalOperator<'a> {
        TraversalOperator {
            mosaic: Arc::clone(self),
            traversal,
        }
    }
}
