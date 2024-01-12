use std::{collections::HashMap, sync::Arc};

use array_tool::vec::Intersect;
use itertools::Itertools;
use mosaic::{
    capabilities::{
        process::ProcessCapability, DictionaryCapability, SelectionCapability, TraversalOperator,
        Traverse,
    },
    internals::{void, EntityId, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile},
    iterators::tile_deletion::TileDeletion,
};
use ordered_multimap::ListOrderedMultimap;

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

pub fn pattern_match(match_process: &Tile) -> anyhow::Result<Tile> {
    let mosaic = Arc::clone(&match_process.mosaic);
    mosaic.new_type("PatternMatchCandidate: s32;")?;
    mosaic.new_type("PatternMatchBinding: s32;")?;

    let pattern_param = mosaic
        .get_process_parameter_value(match_process, "pattern")?
        .unwrap();

    let target_param = mosaic
        .get_process_parameter_value(match_process, "target")?
        .unwrap();

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
        let bindings = mosaic.make_dictionary();
        for (k, v) in result {
            mosaic.add_dictionary_entry(
                &bindings,
                &mosaic.get(k).unwrap(),
                &mosaic.get(v).unwrap(),
            );
        }

        mosaic.add_process_result(match_process, &bindings).unwrap();
    }

    transient.into_iter().delete();

    Ok(match_process.clone())
}

#[cfg(test)]
mod pattern_match_tests {
    use itertools::Itertools;
    use mosaic::{
        capabilities::{process::ProcessCapability, DictionaryCapability, SelectionCapability},
        internals::{void, Mosaic, MosaicCRUD, MosaicIO},
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

        let p = mosaic.make_selection();
        mosaic.fill_selection(&p, &[a, b, c]);

        let t = mosaic.make_selection();
        mosaic.fill_selection(&t, &[g, h, i, j, k]);

        let mtch = mosaic
            .create_process("PatternMatch", &["pattern", "target"])
            .unwrap();
        mosaic.pass_process_parameter(&mtch, "pattern", &p).unwrap();
        mosaic.pass_process_parameter(&mtch, "target", &t).unwrap();
        pattern_match(&mtch).unwrap();

        let results = mosaic.get_process_results(&mtch).unwrap();
        assert_eq!(2, results.len());

        for result in results {
            let binding = mosaic.get_dictionary_entries(&result);
            for k in binding.keys().sorted_by(|&a, &b| a.id.cmp(&b.id)) {
                let v = binding.get(k).unwrap();
                println!("\t{:?} = {:?}; ", chr(k.id), chr(v.id));
            }
            println!();
        }
    }
}
