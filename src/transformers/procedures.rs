use std::{collections::HashMap, sync::Arc};

use mosaic::{
    internals::{
        par, pars, ComponentValuesBuilderSetter, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD,
        Tile,
    },
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

pub trait Procedure {
    fn make_procedure(&self, name: &str) -> ProcedureTile;
    fn add_argument(&self, proc: &Tile, name: &str, tile: &Tile);
    fn get_argument(&self, proc: &Tile, name: &str) -> Option<Tile>;
    fn get_arguments(&self, proc: &Tile) -> HashMap<String, Tile>;
    fn add_result(&self, proc: &Tile, name: &str, result: &Tile);
    fn get_results(&self, proc: &Tile) -> HashMap<String, Tile>;
    fn result_count(&self, proc: &Tile) -> usize;
}

pub struct ProcedureTile(pub Tile);

impl ProcedureTile {
    pub fn add_argument(&self, name: &str, tile: &Tile) {
        self.0.mosaic.add_argument(&self.0, name, tile);
    }

    pub fn get_argument(&self, name: &str) -> Option<Tile> {
        self.0.mosaic.get_argument(&self.0, name)
    }

    pub fn get_arguments(&self) -> HashMap<String, Tile> {
        self.0.mosaic.get_arguments(&self.0)
    }

    pub fn add_result(&self, name: &str, result: &Tile) {
        self.0.mosaic.add_result(&self.0, name, result);
    }

    pub fn get_results(&self) -> HashMap<String, Tile> {
        self.0.mosaic.get_results(&self.0)
    }

    pub fn result_count(&self) -> usize {
        self.0.mosaic.result_count(&self.0)
    }
}

impl Procedure for Arc<Mosaic> {
    fn make_procedure(&self, name: &str) -> ProcedureTile {
        self.new_type("Procedure: str;").unwrap();
        self.new_type("ProcedureArgument: { name: s32, value: u64 };")
            .unwrap();
        self.new_type("ProcedureResult: s32;").unwrap();

        ProcedureTile(self.new_object("Procedure", par(name.to_string())))
    }

    fn add_argument(&self, proc: &Tile, name: &str, tile: &Tile) {
        self.new_descriptor(
            proc,
            "ProcedureArgument",
            pars().set("name", name).set("value", tile.id as u64).ok(),
        );
    }

    fn get_argument(&self, proc: &Tile, name: &str) -> Option<Tile> {
        if let Some(id) = proc
            .iter()
            .get_descriptors()
            .include_component("ProcedureArgument")
            .find(|p| p.get("name").as_s32().is(name))
            .map(|f| f.get("value").as_u64())
        {
            proc.mosaic.get(id as usize)
        } else {
            None
        }
    }

    fn get_arguments(&self, proc: &Tile) -> HashMap<String, Tile> {
        let mut args = HashMap::new();
        for ext in proc
            .iter()
            .get_descriptors()
            .include_component("ProcedureResult")
        {
            args.insert(
                ext.get("name").as_s32().to_string(),
                proc.mosaic.get(ext.get("value").as_u64() as usize).unwrap(),
            );
        }

        args
    }

    fn add_result(&self, proc: &Tile, name: &str, result: &Tile) {
        println!("Adding result: {:?} -> {:?}", name, result);
        proc.arrow_to(result, "ProcedureResult", par(name));
    }

    fn get_results(&self, proc: &Tile) -> HashMap<String, Tile> {
        let mut res = HashMap::new();
        for arrow in proc
            .iter()
            .get_arrows()
            .include_component("ProcedureResult")
        {
            res.insert(arrow.get("self").as_s32().to_string(), arrow.target());
        }

        res
    }

    fn result_count(&self, proc: &Tile) -> usize {
        proc.iter()
            .get_arrows()
            .include_component("ProcedureResult")
            .count()
    }
}
