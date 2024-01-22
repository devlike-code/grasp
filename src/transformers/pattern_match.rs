use std::{collections::HashMap, sync::Arc};

use array_tool::vec::Intersect;
use itertools::Itertools;

use crate::{
    core::{
        structures::errors::ErrorCapability,
        structures::{lists::ListTile, pairs::PairCapability, ListCapability, PairTile},
    },
    querying::traversal::{TraversalOperator, Traverse},
};
use mosaic::iterators::tile_deletion::TileDeletion;
use mosaic::{
    capabilities::SelectionCapability,
    internals::{void, EntityId, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile},
};
use ordered_multimap::ListOrderedMultimap;

use super::ProcedureTile;

#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct PatternMatchState2 {
    candidates: ListOrderedMultimap<EntityId, EntityId>,
    pattern_candidates: ListOrderedMultimap<EntityId, EntityId>,
    candidate_mapping: HashMap<EntityId, (EntityId, EntityId)>,
    rev_candidate_mapping: HashMap<(EntityId, EntityId), EntityId>,
}

fn find_candidates_by_degrees2(
    pattern: &TraversalOperator,
    target: &TraversalOperator,
) -> PatternMatchState2 {
    let mut state = PatternMatchState2::default();
    let mut in_degree_mmap = ListOrderedMultimap::new();
    let mut out_degree_mmap = ListOrderedMultimap::new();

    for target_node in target.get_objects() {
        let in_degree = target.in_degree(&target_node);
        let out_degree = target.out_degree(&target_node);

        for i in 0..=in_degree {
            in_degree_mmap.append(i, target_node.id);
        }

        for i in 0..=out_degree {
            out_degree_mmap.append(i, target_node.id);
        }
    }

    for pattern_node in pattern.get_objects() {
        let in_degree = pattern.in_degree(&pattern_node);
        let out_degree = pattern.out_degree(&pattern_node);

        let in_candidates = in_degree_mmap.get_all(&in_degree).collect_vec();
        let out_candidates = out_degree_mmap.get_all(&out_degree).collect_vec();

        in_candidates
            .intersect(out_candidates)
            .into_iter()
            .for_each(|target_node| {
                state.candidates.append(pattern_node.id, *target_node);
            });
    }

    state
}

#[allow(dead_code)]
fn assign_candidate_and_test2(
    mosaic: Arc<Mosaic>,
    pattern: &TraversalOperator,
    state: &PatternMatchState2,
    remaining_candidates: &[EntityId],
    bindings: &mut Vec<PairTile>,
    results: &mut ListTile,
) {
    if let Some((head, tail)) = remaining_candidates.split_first() {
        for binding in state.pattern_candidates.get_all(head) {
            bindings.push(
                mosaic.make_pair(&mosaic.get(*head).unwrap(), &mosaic.get(*binding).unwrap()),
            );
            assign_candidate_and_test2(
                Arc::clone(&mosaic),
                pattern,
                state,
                tail,
                bindings,
                results,
            );
            bindings.remove(*head);
        }
    } else {
        let traversal = mosaic.traverse(
            bindings
                .iter()
                .map(|pair| mosaic.get(pair.0.id).unwrap())
                .collect_vec()
                .into(),
        );

        let candidates_found = find_candidates_by_degrees2(pattern, &traversal)
            .candidates
            .keys_len();

        if candidates_found == bindings.len() {
            println!("RESULT! {:?}", bindings);
            let result = mosaic.make_list();
            for var in bindings {
                println!("VAR: {:?}", var);
                result.add_back(&var);
            }
            results.add_back(&result);
        }
    }
}

#[allow(dead_code)]
pub fn pattern_match_2(match_process: &ProcedureTile) {
    let mosaic = Arc::clone(&match_process.0.mosaic);
    mosaic.new_type("PatternMatchCandidate: s32;").unwrap();
    mosaic.new_type("PatternMatchBinding: s32;").unwrap();

    let pattern_param = match_process.get_argument("pattern");
    if pattern_param.is_none() {
        mosaic.make_error("Pattern match pattern not correct", None, None);
        return;
    }

    let target_param = match_process.get_argument("target");
    if pattern_param.is_none() {
        mosaic.make_error("Target match pattern not correct", None, None);
        return;
    }

    let pattern_tiles_iter = mosaic.get_selection(&pattern_param.unwrap());
    let target_tiles_iter = mosaic.get_selection(&target_param.unwrap());

    let pattern = mosaic.traverse(pattern_tiles_iter.into());
    let target = mosaic.traverse(target_tiles_iter.into());

    let mut state = find_candidates_by_degrees2(&pattern, &target);

    let reachability = target.as_matrix();

    let mut transient = vec![];

    for start_node in pattern.get_objects() {
        let pid = start_node.id;
        let start_candidates = state.candidates.get_all(&start_node.id).collect_vec();

        for &sc in &start_candidates {
            let candidate = mosaic.new_object("PatternMatchCandidate", void());
            state.candidate_mapping.insert(candidate.id, (pid, *sc));
            state.rev_candidate_mapping.insert((pid, *sc), candidate.id);
            state.pattern_candidates.append(pid, candidate.id);
            transient.push(candidate);
        }
    }

    for start_node in pattern.get_objects() {
        let pid = start_node.id;
        let start_candidates = state.candidates.get_all(&start_node.id).collect_vec();

        for end_node in pattern.get_forward_neighbors(&start_node) {
            let tid = end_node.id;
            let end_candidates = state.candidates.get_all(&end_node.id).collect_vec();

            for &sc in &start_candidates {
                for &ec in &end_candidates {
                    if *sc == *ec {
                        continue;
                    }

                    if !reachability.are_adjacent(*sc, *ec) {
                        continue;
                    }

                    let cand1 = state.rev_candidate_mapping.get(&(pid, *sc)).unwrap();
                    let cand2 = state.rev_candidate_mapping.get(&(tid, *ec)).unwrap();

                    let binding = mosaic.new_arrow(cand1, cand2, "PatternMatchBinding", void());

                    transient.push(binding);
                }
            }
        }
    }

    let keys = state.pattern_candidates.keys().cloned().collect_vec();

    let mut results = mosaic.make_list();
    assign_candidate_and_test2(
        Arc::clone(&mosaic),
        &pattern,
        &state,
        &keys,
        &mut Vec::new(),
        &mut results,
    );

    // let bindings = mosaic.make_list();
    // for result in results.iter() {
    //     bindings.add_back(&result);
    // }
    // match_process.add_result(
    //     format!("{}", match_process.result_count()).as_str(),
    //     &bindings.into(),
    // );
}

#[cfg(test)]
mod pattern_match_tests_2 {
    use itertools::Itertools;
    use mosaic::{
        capabilities::SelectionCapability,
        internals::{void, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD},
        iterators::tile_getters::TileGetters,
    };

    use crate::{
        core::structures::ListTile,
        editor_state::foundation::GraspEditorState,
        transformers::{pattern_match_2, Procedure},
    };

    // use super::pattern_match;

    #[allow(dead_code)]
    fn chr(i: usize) -> char {
        char::from_u32(65 + i as u32).unwrap()
    }

    #[test]
    fn test_pattern_match() {
        let mosaic = Mosaic::new();

        let a = mosaic.new_object("void", void()); // 0
        let b = mosaic.new_object("void", void()); // 1
        let c = mosaic.new_object("void", void()); // 2
        mosaic.new_arrow(&a, &b, "void", void()); // 3
        mosaic.new_arrow(&a, &c, "void", void()); // 4
        mosaic.new_arrow(&b, &c, "void", void()); // 5

        let g = mosaic.new_object("void", void()); // 6
        let h = mosaic.new_object("void", void()); // 7
        let i = mosaic.new_object("void", void()); // 8
        let j = mosaic.new_object("void", void()); // 9
        let k = mosaic.new_object("void", void()); // 10
        mosaic.new_arrow(&g, &h, "void", void()); // 11
        mosaic.new_arrow(&g, &i, "void", void()); // 12
        mosaic.new_arrow(&h, &i, "void", void()); // 13
        mosaic.new_arrow(&g, &j, "void", void()); // 14
        mosaic.new_arrow(&i, &j, "void", void()); // 15
        mosaic.new_arrow(&h, &k, "void", void()); // 16

        let p = mosaic.make_selection(&[a, b, c]);
        let t = mosaic.make_selection(&[g, h, i, j, k]);

        let mtch = mosaic.make_procedure("PatternMatch");
        mtch.add_argument("pattern", &p);
        mtch.add_argument("target", &t);
        pattern_match_2(&mtch);

        // let results = mtch.get_results();
        // println!(
        //     "{:?}",
        //     ListTile::from_tile(results.get("0").unwrap().clone()).iter()
        // );
        // //assert_eq!(2, results.len());

        // for (name, binding) in results {
        //     if let Some(binding) = ListTile::from_tile(binding) {
        //         println!("{:?}", binding.iter());
        //         for k in binding.iter().sorted_by(|a, b| a.id.cmp(&b.id)) {
        //             println!("\t{:?} = {:?}; ", name, chr(k.id));
        //         }
        //         println!();
        //     } else {
        //         println!("No list...");
        //     }
        // }
    }
}

#[derive(Default)]
pub(crate) struct PatternMatchState {
    candidates: ListOrderedMultimap<EntityId, EntityId>,
    pattern_candidates: ListOrderedMultimap<EntityId, EntityId>,
    candidate_mapping: HashMap<EntityId, (EntityId, EntityId)>,
    rev_candidate_mapping: HashMap<(EntityId, EntityId), EntityId>,
}

fn find_candidates_by_degrees(
    pattern: &TraversalOperator,
    target: &TraversalOperator,
) -> PatternMatchState {
    let mut state = PatternMatchState::default();
    let mut in_degree_mmap = ListOrderedMultimap::new();
    let mut out_degree_mmap = ListOrderedMultimap::new();

    for target_node in target.get_objects() {
        let in_degree = target.in_degree(&target_node);
        let out_degree = target.out_degree(&target_node);

        for i in 0..=in_degree {
            in_degree_mmap.append(i, target_node.id);
        }

        for i in 0..=out_degree {
            out_degree_mmap.append(i, target_node.id);
        }
    }

    for pattern_node in pattern.get_objects() {
        let in_degree = pattern.in_degree(&pattern_node);
        let out_degree = pattern.out_degree(&pattern_node);

        let in_candidates = in_degree_mmap.get_all(&in_degree).collect_vec();
        let out_candidates = out_degree_mmap.get_all(&out_degree).collect_vec();

        in_candidates
            .intersect(out_candidates)
            .into_iter()
            .for_each(|target_node| {
                state.candidates.append(pattern_node.id, *target_node);
            });
    }

    state
}

fn assign_candidate_and_test(
    mosaic: Arc<Mosaic>,
    pattern: &TraversalOperator,
    state: &PatternMatchState,
    remaining_candidates: &[EntityId],
    bindings: &mut HashMap<EntityId, EntityId>,
    results: &mut Vec<HashMap<EntityId, EntityId>>,
) {
    if let Some((head, tail)) = remaining_candidates.split_first() {
        for binding in state.pattern_candidates.get_all(head) {
            bindings.insert(*head, *binding);
            assign_candidate_and_test(Arc::clone(&mosaic), pattern, state, tail, bindings, results);
            bindings.remove(head);
        }
    } else {
        let traversal = mosaic.traverse(
            bindings
                .values()
                .map(|id| mosaic.get(*id).unwrap())
                .collect_vec()
                .into(),
        );

        let candidates_found = find_candidates_by_degrees(pattern, &traversal)
            .candidates
            .keys_len();

        if candidates_found == bindings.len() {
            results.push(HashMap::from_iter(
                bindings
                    .iter()
                    .map(|(k, v)| (*k, state.candidate_mapping.get(v).unwrap().1))
                    .collect_vec(),
            ));
        }
    }
}

pub fn pattern_match(match_process: &ProcedureTile) -> anyhow::Result<Tile> {
    let mosaic = Arc::clone(&match_process.0.mosaic);
    mosaic.new_type("PatternMatchCandidate: s32;")?;
    mosaic.new_type("PatternMatchBinding: s32;")?;

    let pattern_param = match_process.get_argument("pattern").unwrap();
    let target_param = match_process.get_argument("target").unwrap();

    let pattern_tiles_iter = mosaic.get_selection(&pattern_param);
    let target_tiles_iter = mosaic.get_selection(&target_param);

    let pattern = mosaic.traverse(pattern_tiles_iter.into());
    let target = mosaic.traverse(target_tiles_iter.into());

    let mut state = find_candidates_by_degrees(&pattern, &target);

    let reachability = target.as_matrix();

    let mut transient = vec![];

    for start_node in pattern.get_objects() {
        let pid = start_node.id;
        let start_candidates = state.candidates.get_all(&start_node.id).collect_vec();

        for &sc in &start_candidates {
            let candidate = mosaic.new_object("PatternMatchCandidate", void());
            state.candidate_mapping.insert(candidate.id, (pid, *sc));
            state.rev_candidate_mapping.insert((pid, *sc), candidate.id);
            state.pattern_candidates.append(pid, candidate.id);
            transient.push(candidate);
        }
    }

    for start_node in pattern.get_objects() {
        let pid = start_node.id;
        let start_candidates = state.candidates.get_all(&start_node.id).collect_vec();

        for end_node in pattern.get_forward_neighbors(&start_node) {
            let tid = end_node.id;
            let end_candidates = state.candidates.get_all(&end_node.id).collect_vec();

            for &sc in &start_candidates {
                for &ec in &end_candidates {
                    if *sc == *ec {
                        continue;
                    }

                    if !reachability.are_adjacent(*sc, *ec) {
                        continue;
                    }

                    let cand1 = state.rev_candidate_mapping.get(&(pid, *sc)).unwrap();
                    let cand2 = state.rev_candidate_mapping.get(&(tid, *ec)).unwrap();

                    let binding = mosaic.new_arrow(cand1, cand2, "PatternMatchBinding", void());

                    transient.push(binding);
                }
            }
        }
    }

    let keys = state.pattern_candidates.keys().cloned().collect_vec();

    let mut results = Vec::new();
    assign_candidate_and_test(
        Arc::clone(&mosaic),
        &pattern,
        &state,
        &keys,
        &mut HashMap::new(),
        &mut results,
    );

    for result in results {
        let bindings = mosaic.make_list();
        for (k, v) in result {
            println!("\tBINDING {} -> {}", k, v);
            let binding_pair = mosaic.make_pair(&k, &v);
            bindings.add_back(&binding_pair);
        }

        match_process.add_result(&bindings);
    }

    transient.into_iter().delete();

    Ok(match_process.0.clone())
}

#[cfg(test)]
mod pattern_match_tests {
    use itertools::Itertools;
    use mosaic::{
        capabilities::SelectionCapability,
        internals::{void, Mosaic, MosaicCRUD, MosaicIO},
    };

    use crate::{
        core::structures::{ListTile, PairTile},
        transformers::Procedure,
    };

    use super::pattern_match;

    fn chr(i: usize) -> char {
        char::from_u32(65 + i as u32).unwrap()
    }

    #[test]
    fn test_pattern_match() {
        let mosaic = Mosaic::new();
        let a = mosaic.new_object("void", void()); // 0
        let b = mosaic.new_object("void", void()); // 1
        let c = mosaic.new_object("void", void()); // 2
        mosaic.new_arrow(&a, &a, "void", void()); // 11
        mosaic.new_arrow(&a, &b, "void", void()); // 3
        mosaic.new_arrow(&a, &c, "void", void()); // 4
        mosaic.new_arrow(&b, &c, "void", void()); // 5

        let g = mosaic.new_object("void", void()); // 6
        let h = mosaic.new_object("void", void()); // 7
        let i = mosaic.new_object("void", void()); // 8
        let j = mosaic.new_object("void", void()); // 9
        let k = mosaic.new_object("void", void()); // 10
        mosaic.new_arrow(&g, &g, "void", void()); // 11
        mosaic.new_arrow(&g, &h, "void", void()); // 11
        mosaic.new_arrow(&g, &i, "void", void()); // 12
        mosaic.new_arrow(&h, &i, "void", void()); // 13
        mosaic.new_arrow(&g, &j, "void", void()); // 14
        mosaic.new_arrow(&i, &j, "void", void()); // 15
        mosaic.new_arrow(&h, &k, "void", void()); // 16

        let p = mosaic.make_selection(&[a, b, c]);
        let t = mosaic.make_selection(&[g, h, i, j, k]);

        let mtch = mosaic.make_procedure("PatternMatch");
        mtch.add_argument("pattern", &p);
        mtch.add_argument("target", &t);
        pattern_match(&mtch).unwrap();

        let results = mtch.get_results();
        assert_eq!(2, results.len());

        for result in results {
            let list = ListTile::from_tile(result).unwrap();
            for binding in list.iter() {
                let bind = PairTile::from_tile(binding).unwrap();
                println!("{:?} -> {:?}", bind.get_first(), bind.get_second());
            }
            println!();
        }
    }
}
