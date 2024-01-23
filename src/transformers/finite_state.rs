use std::{collections::HashMap, fs, sync::Arc};

use array_tool::vec::Uniq;
use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{MosaicIO, Tile},
    iterators::tile_filters::TileFilters,
};

use crate::core::structures::errors::ErrorCapability;

fn make_enum(name: &str, members: &[String]) -> String {
    format!(
        "#[derive(Debug, Clone, Copy)]\npub enum {} {{\n{}\n}}\n",
        name,
        members.iter().map(|m| format!("\t{},", m)).join("\n")
    )
}

pub fn finite_state_transformer(initial_state: &[Tile], window: &Tile) {
    let initial_state = initial_state.first().unwrap();
    let document = Arc::clone(&initial_state.mosaic);
    let mut result = vec![];

    if let Some(fsm) = initial_state.get_component("FSM") {
        let name = fsm.get("self").as_s32().to_string();
        let fsm_name = format!("{}FSM", name);
        let objects_len = document.get_all().filter_objects().len();

        let nodes: HashMap<usize, String> = HashMap::from_iter(
            document
                .get_all()
                .filter_objects()
                .map(|t| {
                    (
                        t.id,
                        t.get_component("Label")
                            .map(|l| l.get("self").as_s32().to_string())
                            .unwrap_or_default(),
                    )
                })
                .filter(|(_, l)| !l.is_empty()),
        );

        if nodes.len() < objects_len {
            window.mosaic.make_error(
                "Not all nodes have a distinct name in this FSM.",
                Some(window.clone()),
                Some(initial_state.clone()),
            );
            return;
        }

        let node_name = format!("{}State", name);
        result.push(make_enum(
            &node_name,
            &nodes.values().cloned().collect_vec(),
        ));

        let mut transitions = vec![];

        let arrows = document
            .get_all()
            .filter_arrows()
            .flat_map(|t| {
                let label = t
                    .get_component("Label")
                    .map(|l| l.get("self").as_s32().to_string());

                transitions.push((
                    nodes.get(&t.source_id()).unwrap(),
                    label.clone().unwrap_or_default(),
                    nodes.get(&t.target_id()).unwrap(),
                ));
                label
            })
            .collect_vec()
            .unique();

        let arrow_name = format!("{}Transition", name);
        result.push(make_enum(&arrow_name, &arrows));

        result.push(format!(
            "pub struct {} {{ state: {}, }}\n",
            fsm_name, node_name
        ));
        result.push(format!("impl FSM for {} {{", fsm_name));
        result.push(format!("\ttype State = {};", node_name));
        result.push(format!("\ttype Transition = {};\n", arrow_name));
        result.push("\tfn get_current_state(&self) -> Self::State { self.state }\n".to_string());
        result.push(
            "\tfn react_to(&self, trigger: Self::Transition) -> Option<Self::State> {".to_string(),
        );
        result.push("\t\tmatch (self.state, trigger) {".to_string());
        for (a, t, b) in transitions {
            result.push(format!(
                "\t\t\t({}::{}, {}::{}) => Some({}::{}),",
                node_name, a, arrow_name, t, node_name, b
            ));
        }
        result.push("\t\t\t_ => None,".to_string());
        result.push("\t\t}".to_string());
        result.push("\t}".to_string());
        result.push("}".to_string());

        println!("\n{}", result.join("\n"));

        if let Some(dir) = initial_state.get_component("OutputDir") {
            println!(
                "Path = {:?}",
                format!(
                    "{}//{}.rs",
                    dir.get("self").as_s32(),
                    fsm_name.to_lowercase()
                )
            );
            let write = fs::write(
                format!(
                    "{}\\{}.rs",
                    dir.get("self").as_s32(),
                    fsm_name.to_lowercase()
                ),
                result.join("\n"),
            );

            if write.is_err() {
                println!("{:?}", write.unwrap_err().to_string());
            }
        }
    }
}

pub trait FSM {
    type State;
    type Transition: Copy + Clone;

    fn get_current_state(&self) -> Self::State;
    fn react_to(&self, trigger: Self::Transition) -> Option<Self::State>;
}
