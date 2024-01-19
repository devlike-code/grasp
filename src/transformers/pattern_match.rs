// use std::{collections::HashMap, sync::Arc};

// use array_tool::vec::Intersect;
// use itertools::Itertools;

// use mosaic::{
//     capabilities::SelectionCapability,
//     internals::{void, EntityId, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD},
// };
// use ordered_multimap::ListOrderedMultimap;

// use crate::{
//     core::{
//         structures::errors::ErrorCapability,
//         structures::{lists::ListTile, pairs::PairCapability, ListCapability, PairTile},
//     },
//     querying::traversal::{TraversalOperator, Traverse},
// };

// use super::ProcedureTile;

// #[allow(dead_code)]
// #[derive(Default)]
// pub(crate) struct PatternMatchState {
//     candidates: ListOrderedMultimap<EntityId, EntityId>,
//     pattern_candidates: ListOrderedMultimap<EntityId, EntityId>,
//     candidate_mapping: HashMap<EntityId, (EntityId, EntityId)>,
//     rev_candidate_mapping: HashMap<(EntityId, EntityId), EntityId>,
// }

// fn find_candidates_by_degrees(
//     pattern: &TraversalOperator,
//     target: &TraversalOperator,
// ) -> PatternMatchState {
//     let mut state = PatternMatchState::default();
//     let mut in_degree_mmap = ListOrderedMultimap::new();
//     let mut out_degree_mmap = ListOrderedMultimap::new();

//     for target_node in target.get_objects() {
//         let in_degree = target.in_degree(&target_node);
//         let out_degree = target.out_degree(&target_node);

//         for i in 0..=in_degree {
//             in_degree_mmap.append(i, target_node.id);
//         }

//         for i in 0..=out_degree {
//             out_degree_mmap.append(i, target_node.id);
//         }
//     }

//     for pattern_node in pattern.get_objects() {
//         let in_degree = pattern.in_degree(&pattern_node);
//         let out_degree = pattern.out_degree(&pattern_node);

//         let in_candidates = in_degree_mmap.get_all(&in_degree).collect_vec();
//         let out_candidates = out_degree_mmap.get_all(&out_degree).collect_vec();

//         in_candidates
//             .intersect(out_candidates)
//             .into_iter()
//             .for_each(|target_node| {
//                 state.candidates.append(pattern_node.id, *target_node);
//             });
//     }

//     state
// }

// #[allow(dead_code)]
// fn assign_candidate_and_test(
//     mosaic: Arc<Mosaic>,
//     pattern: &TraversalOperator,
//     state: &PatternMatchState,
//     remaining_candidates: &[EntityId],
//     bindings: &mut Vec<PairTile>,
//     results: &mut ListTile,
// ) {
//     if let Some((head, tail)) = remaining_candidates.split_first() {
//         for binding in state.pattern_candidates.get_all(head) {
//             bindings.push(
//                 mosaic.make_pair(&mosaic.get(*head).unwrap(), &mosaic.get(*binding).unwrap()),
//             );
//             assign_candidate_and_test(Arc::clone(&mosaic), pattern, state, tail, bindings, results);
//             bindings.remove(*head);
//         }
//     } else {
//         let traversal = mosaic.traverse(
//             bindings
//                 .iter()
//                 .map(|pair| mosaic.get(pair.0.id).unwrap())
//                 .collect_vec()
//                 .into(),
//         );

//         let candidates_found = find_candidates_by_degrees(pattern, &traversal)
//             .candidates
//             .keys_len();

//         if candidates_found == bindings.len() {
//             println!("RESULT! {:?}", bindings);
//             let result = mosaic.make_list();
//             for var in bindings {
//                 result.add_back(&var.0);
//             }
//             results.add_back(&result.0);
//         }
//     }
// }

// #[allow(dead_code)]
// pub fn pattern_match(match_process: &ProcedureTile) {
//     let mosaic = Arc::clone(&match_process.0.mosaic);
//     mosaic.new_type("PatternMatchCandidate: s32;").unwrap();
//     mosaic.new_type("PatternMatchBinding: s32;").unwrap();

//     let pattern_param = match_process.get_argument("pattern");
//     if pattern_param.is_none() {
//         mosaic.make_error("Pattern match pattern not correct", None, None);
//         return;
//     }

//     let target_param = match_process.get_argument("target");
//     if pattern_param.is_none() {
//         mosaic.make_error("Target match pattern not correct", None, None);
//         return;
//     }

//     let pattern_tiles_iter = mosaic.get_selection(&pattern_param.unwrap());
//     let target_tiles_iter = mosaic.get_selection(&target_param.unwrap());

//     let pattern = mosaic.traverse(pattern_tiles_iter.into());
//     let target = mosaic.traverse(target_tiles_iter.into());

//     let mut state = find_candidates_by_degrees(&pattern, &target);

//     let reachability = target.as_matrix();

//     let mut transient = vec![];

//     for start_node in pattern.get_objects() {
//         let pid = start_node.id;
//         let start_candidates = state.candidates.get_all(&start_node.id).collect_vec();

//         for &sc in &start_candidates {
//             let candidate = mosaic.new_object("PatternMatchCandidate", void());
//             state.candidate_mapping.insert(candidate.id, (pid, *sc));
//             state.rev_candidate_mapping.insert((pid, *sc), candidate.id);
//             state.pattern_candidates.append(pid, candidate.id);
//             transient.push(candidate);
//         }
//     }

//     for start_node in pattern.get_objects() {
//         let pid = start_node.id;
//         let start_candidates = state.candidates.get_all(&start_node.id).collect_vec();

//         for end_node in pattern.get_forward_neighbors(&start_node) {
//             let tid = end_node.id;
//             let end_candidates = state.candidates.get_all(&end_node.id).collect_vec();

//             for &sc in &start_candidates {
//                 for &ec in &end_candidates {
//                     if *sc == *ec {
//                         continue;
//                     }

//                     if !reachability.are_adjacent(*sc, *ec) {
//                         continue;
//                     }

//                     let cand1 = state.rev_candidate_mapping.get(&(pid, *sc)).unwrap();
//                     let cand2 = state.rev_candidate_mapping.get(&(tid, *ec)).unwrap();

//                     let binding = mosaic.new_arrow(cand1, cand2, "PatternMatchBinding", void());

//                     transient.push(binding);
//                 }
//             }
//         }
//     }

//     let keys = state.pattern_candidates.keys().cloned().collect_vec();

//     let mut results = mosaic.make_list();
//     assign_candidate_and_test(
//         Arc::clone(&mosaic),
//         &pattern,
//         &state,
//         &keys,
//         &mut Vec::new(),
//         &mut results,
//     );

//     let bindings = mosaic.make_list();
//     for result in results.iter() {
//         bindings.add_back(&result);
//     }
//     match_process.add_result(
//         format!("{}", match_process.result_count()).as_str(),
//         &bindings.into(),
//     );
// }

// #[cfg(test)]
// mod pattern_match_tests {
//     use itertools::Itertools;
//     use mosaic::{
//         capabilities::SelectionCapability,
//         internals::{void, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD},
//     };

//     use crate::{
//         core::structures::ListTile,
//         editor_state::foundation::GraspEditorState,
//         transformers::{pattern_match, Procedure},
//     };

//     // use super::pattern_match;

//     #[allow(dead_code)]
//     fn chr(i: usize) -> char {
//         char::from_u32(65 + i as u32).unwrap()
//     }

//     #[test]
//     fn test_pattern_match() {
//         let mosaic = Mosaic::new();

//         let a = mosaic.new_object("void", void()); // 0
//         let b = mosaic.new_object("void", void()); // 1
//         let c = mosaic.new_object("void", void()); // 2
//         mosaic.new_arrow(&a, &b, "void", void()); // 3
//         mosaic.new_arrow(&a, &c, "void", void()); // 4
//         mosaic.new_arrow(&b, &c, "void", void()); // 5

//         let g = mosaic.new_object("void", void()); // 6
//         let h = mosaic.new_object("void", void()); // 7
//         let i = mosaic.new_object("void", void()); // 8
//         let j = mosaic.new_object("void", void()); // 9
//         let k = mosaic.new_object("void", void()); // 10
//         mosaic.new_arrow(&g, &h, "void", void()); // 11
//         mosaic.new_arrow(&g, &i, "void", void()); // 12
//         mosaic.new_arrow(&h, &i, "void", void()); // 13
//         mosaic.new_arrow(&g, &j, "void", void()); // 14
//         mosaic.new_arrow(&i, &j, "void", void()); // 15
//         mosaic.new_arrow(&h, &k, "void", void()); // 16

//         let p = mosaic.make_selection(&[a, b, c]);
//         let t = mosaic.make_selection(&[g, h, i, j, k]);

//         let mtch = mosaic.make_procedure("PatternMatch");
//         mtch.add_argument("pattern", &p);
//         mtch.add_argument("target", &t);
//         pattern_match(&mtch);

//         let results = mtch.get_results();
//         println!("{:?}", results);
//         //assert_eq!(2, results.len());

//         for (name, binding) in results {
//             if let Some(binding) = ListTile::from_tile(binding) {
//                 println!("{:?}", binding.iter());
//                 for k in binding.iter().sorted_by(|a, b| a.id.cmp(&b.id)) {
//                     println!("\t{:?} = {:?}; ", name, chr(k.id));
//                 }
//                 println!();
//             } else {
//                 println!("No list...");
//             }
//         }
//     }
// }
