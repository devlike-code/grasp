use std::{borrow::Borrow, collections::HashMap, sync::Arc};

use array_tool::vec::Intersect;
use itertools::Itertools;

use crate::{
    core::{
        has_mosaic::HasMosaic,
        structures::{pairs::PairCapability, ErrorCapability, ListCapability, ListTile, PairTile},
    },
    editor_state::{foundation::TransformerState, windows::GraspEditorWindow},
    querying::traversal::{TraversalOperator, Traverse},
    GuiState,
};
use mosaic::iterators::{component_selectors::ComponentSelectors, tile_deletion::TileDeletion};
use mosaic::{
    capabilities::SelectionCapability,
    internals::{void, EntityId, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile},
};
use ordered_multimap::ListOrderedMultimap;

use super::{Procedure, ProcedureTile};

#[derive(Default, Debug)]
pub(crate) struct PatternMatchState {
    candidates: ListOrderedMultimap<EntityId, EntityId>,
    pattern_candidates: ListOrderedMultimap<EntityId, EntityId>,
    candidate_mapping: HashMap<EntityId, (EntityId, EntityId)>,
    rev_candidate_mapping: HashMap<(EntityId, EntityId), EntityId>,
    loops: HashMap<EntityId, usize>,
}

fn find_candidates_by_degrees(
    pattern: &TraversalOperator,
    target: &TraversalOperator,
) -> PatternMatchState {
    let mut state = PatternMatchState::default();
    let mut in_degree_mmap = ListOrderedMultimap::new();
    let mut out_degree_mmap = ListOrderedMultimap::new();
    let mut loop_degree_mmap = ListOrderedMultimap::new();

    for target_node in target.get_objects() {
        let loop_degree = target.get_self_loops(&target_node).len();
        let in_degree = target.in_degree(&target_node) - loop_degree;
        let out_degree = target.out_degree(&target_node) - loop_degree;

        println!(
            "\t\tTARGET {:?} {} {} {}",
            target_node, in_degree, out_degree, loop_degree
        );

        for i in 0..=in_degree {
            in_degree_mmap.append(i, target_node.id);
        }

        for i in 0..=out_degree {
            out_degree_mmap.append(i, target_node.id);
        }

        for i in 0..=loop_degree {
            loop_degree_mmap.append(i, target_node.id);
        }

        state.loops.insert(target_node.id, loop_degree);
    }
    println!("\t\t-----------------------",);
    for pattern_node in pattern.get_objects() {
        let loops = pattern.get_self_loops(&pattern_node);
        let loop_degree = loops.len();
        let in_degree = pattern.in_degree(&pattern_node) - loop_degree;
        let out_degree = pattern.out_degree(&pattern_node) - loop_degree;

        println!(
            "\t\t{:?} {} {} {}",
            pattern_node, in_degree, out_degree, loop_degree
        );
        let in_candidates = in_degree_mmap.get_all(&in_degree).collect_vec();
        let out_candidates = out_degree_mmap.get_all(&out_degree).collect_vec();
        let loop_candidates = loop_degree_mmap.get_all(&loop_degree).collect_vec();
        println!("\t\tIN CAND:   {:?}", in_candidates);
        println!("\t\tOUT CAND:  {:?}", out_candidates);
        println!("\t\tLOOP CAND: {:?}", loop_candidates);

        in_candidates
            .intersect(out_candidates)
            .intersect(loop_candidates)
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
    println!(
        "ASSIGN CANDIDATE AND TEST: {:?} {:?}\nREMAINING: {:?}\n",
        bindings, results, remaining_candidates
    );
    if let Some((head, tail)) = remaining_candidates.split_first() {
        for binding in state.pattern_candidates.get_all(head) {
            bindings.insert(*head, *binding);
            assign_candidate_and_test(Arc::clone(&mosaic), pattern, state, tail, bindings, results);
            bindings.remove(head);
        }
    } else {
        println!(
            "\n\t*********** NO REMAINING CANDIDATES. TESTING {:?}",
            bindings
        );
        let traversal = mosaic.traverse(
            bindings
                .values()
                .map(|id| mosaic.get(*id).unwrap())
                .collect_vec()
                .into(),
        );

        let candidates = find_candidates_by_degrees(pattern, &traversal).candidates;
        let candidates_found = candidates.keys_len();
        println!("\nBY DEGREES ({}): {:?}", candidates_found, candidates);

        if candidates_found == bindings.len() {
            results.push(HashMap::from_iter(
                bindings
                    .iter()
                    .map(|(k, v)| (*k, state.candidate_mapping.get(v).unwrap().1))
                    .collect_vec(),
            ));

            println!("\tRESULTS FOUND: {:?}", bindings,);
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
            for _ in 0..*state.loops.get(sc).unwrap() {
                mosaic.new_arrow(&candidate, &candidate, "void", void());
            }
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
    use mosaic::{
        capabilities::SelectionCapability,
        internals::{void, Mosaic, MosaicCRUD, MosaicIO},
    };

    use crate::{
        core::structures::{ListTile, PairTile},
        transformers::Procedure,
    };

    use super::pattern_match;

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

    #[test]
    fn test_pattern_match_with_loop_on_a() {
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
        let l = mosaic.new_arrow(&a, &a, "void", void()); // 17
        assert!(l.is_loop());
        let l1 = mosaic.new_arrow(&g, &g, "void", void()); // 18
        assert!(l1.is_loop());

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

    #[test]
    fn test_pattern_match_with_loop_on_a_and_hj_arrow() {
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
        let l = mosaic.new_arrow(&a, &a, "void", void()); // 17
        assert!(l.is_loop());
        let l1 = mosaic.new_arrow(&g, &g, "void", void()); // 18
        assert!(l1.is_loop());
        mosaic.new_arrow(&h, &j, "void", void());

        let p = mosaic.make_selection(&[a, b, c]);
        let t = mosaic.make_selection(&[g, h, i, j, k]);

        let mtch = mosaic.make_procedure("PatternMatch");
        mtch.add_argument("pattern", &p);
        mtch.add_argument("target", &t);
        pattern_match(&mtch).unwrap();

        let results = mtch.get_results();
        assert_eq!(3, results.len());

        for result in results {
            let list = ListTile::from_tile(result).unwrap();
            for binding in list.iter() {
                let bind = PairTile::from_tile(binding).unwrap();
                println!("{:?} -> {:?}", bind.get_first(), bind.get_second());
            }
            println!();
        }
    }

    #[test]
    fn test_pattern_match_with_loops_on_ah_and_hj_arrow() {
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
        let l = mosaic.new_arrow(&a, &a, "void", void()); // 17
        assert!(l.is_loop());
        let l1 = mosaic.new_arrow(&h, &h, "void", void()); // 17
        assert!(l1.is_loop());
        let l2 = mosaic.new_arrow(&g, &g, "void", void()); // 18
        assert!(l2.is_loop());
        mosaic.new_arrow(&h, &j, "void", void());

        let p = mosaic.make_selection(&[a, b, c]);
        let t = mosaic.make_selection(&[g, h, i, j, k]);

        let mtch = mosaic.make_procedure("PatternMatch");
        mtch.add_argument("pattern", &p);
        mtch.add_argument("target", &t);
        pattern_match(&mtch).unwrap();

        let results = mtch.get_results();
        assert_eq!(4, results.len());

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

pub fn pattern_match_validation(window: &GraspEditorWindow, ui: &GuiState) -> TransformerState {
    ui.window("Pattern Match")
        .build(|| {
            let pick1 = window
                .document_mosaic
                .get_all()
                .include_component("Pick1")
                .next();
            ui.text(format!(
                "Pattern (pick #1): {}",
                pick1
                    .as_ref()
                    .map(|t| t.id.to_string())
                    .unwrap_or("Nothing".to_string())
            ));
            ui.spacing();
            let pick2 = window
                .document_mosaic
                .get_all()
                .include_component("Pick2")
                .next();
            ui.text(format!(
                "Target (pick #2): {}",
                pick2
                    .as_ref()
                    .map(|t| t.id.to_string())
                    .unwrap_or("Nothing".to_string())
            ));

            ui.separator();
            let token = if pick1.is_none() || pick2.is_none() {
                Some(ui.begin_disabled(true))
            } else {
                None
            };
            if ui.button_with_size("Run", [100.0, 20.0]) {
                let p = window.document_mosaic.make_procedure("PatternMatch");
                p.add_argument("pattern", &pick1.unwrap().target());
                p.add_argument("target", &pick2.unwrap().target());
                match pattern_match(&p) {
                    Ok(_) => {
                        println!("PATTERN MATCH OK!");
                        for result in p.get_results() {
                            let list = ListTile::from_tile(result).unwrap();
                            for binding in list.iter() {
                                let bind = PairTile::from_tile(binding).unwrap();
                                println!("{:?} -> {:?}", bind.get_first(), bind.get_second());
                            }
                            println!();
                        }
                    }
                    Err(e) => {
                        println!("PATTERN MATCH ERROR: {:?}!", e.to_string());
                        window.editor_mosaic.make_error(
                            &e.to_string(),
                            Some(window.window_tile.clone()),
                            Some(p.0),
                        );
                    }
                }

                if let Some(t) = token {
                    t.end()
                }
                return TransformerState::Valid;
            }
            if let Some(t) = token {
                t.end()
            }

            ui.same_line();
            if ui.button_with_size("Cancel", [100.0, 20.0]) {
                return TransformerState::Cancelled;
            }

            TransformerState::Running
        })
        .unwrap_or(TransformerState::Running)
}

pub fn pattern_match_tool(initial_state: &[Tile], _window: &Tile) {}
