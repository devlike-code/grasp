use std::sync::Arc;

use mosaic::{
    capabilities::QueueCapability,
    internals::{Mosaic, MosaicIO, Tile},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::core::has_mosaic::HasMosaic;

pub trait GraspQueue {
    fn get_queue_name(&self) -> String;

    fn get_queue_tile(&self, mosaic: &Arc<Mosaic>) -> Tile {
        mosaic
            .get_all()
            .include_component(self.get_queue_name().as_str())
            .get_targets()
            .next()
            .unwrap()
    }
}

pub fn enqueue_direct(queue: Tile, message: Tile) {
    let mosaic = message.get_mosaic();
    mosaic.enqueue(&queue, &message);
}

pub fn enqueue<Q: GraspQueue>(queue: Q, message: Tile) {
    let mosaic = message.get_mosaic();
    let tile = queue.get_queue_tile(&mosaic);
    mosaic.enqueue(&tile, &message);
}

pub fn peek<Q: GraspQueue>(queue: Q, mosaic: &Arc<Mosaic>) -> Option<Tile> {
    let tile = queue.get_queue_tile(mosaic);
    mosaic.peek_queue(&tile)
}

pub fn is_empty<Q: GraspQueue>(queue: Q, mosaic: &Arc<Mosaic>) -> bool {
    peek(queue, mosaic).is_none()
}

pub fn dequeue<Q: GraspQueue>(queue: Q, mosaic: &Arc<Mosaic>) -> Option<Tile> {
    let tile = queue.get_queue_tile(mosaic);
    mosaic.dequeue(&tile)
}
