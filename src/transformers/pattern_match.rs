use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use array_tool::vec::Intersect;
use imgui::{DrawListMut, ImColor32, TreeNodeFlags};
use itertools::Itertools;
use log::warn;

use crate::{
    core::{
        gui::windowing::gui_draw_image,
        math::Vec2,
        structures::{pairs::PairCapability, ErrorCapability, ListCapability, ListTile, PairTile},
    },
    editor_state::{
        foundation::TransformerState, selection::SelectionTile, windows::GraspEditorWindow,
    },
    grasp_render::{
        default_renderer_draw_arrow, default_renderer_draw_object, draw_arrow, draw_node,
    },
    grasp_transitions::query_position_recursive,
    querying::{
        traversal::{TraversalOperator, Traverse},
        Collage, MosaicCollage, Traversal,
    },
    utilities::{ColorQuery, OffsetQuery, PosQuery},
    GuiState,
};
use mosaic::{
    capabilities::{ArchetypeSubject, SelectionCapability},
    internals::{
        par, pars, void, ComponentValuesBuilderSetter, EntityId, Mosaic, MosaicCRUD, MosaicIO,
        MosaicTypelevelCRUD, Tile,
    },
    iterators::tile_getters::TileGetters,
};
use mosaic::{
    internals::TileFieldEmptyQuery,
    iterators::{component_selectors::ComponentSelectors, tile_deletion::TileDeletion},
};
use ordered_multimap::ListOrderedMultimap;

use super::{procedure_args_renderer, Procedure, ProcedureTile};

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

        warn!(
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
    warn!("\t\t-----------------------",);
    for pattern_node in pattern.get_objects() {
        let loops = pattern.get_self_loops(&pattern_node);
        let loop_degree = loops.len();
        let in_degree = pattern.in_degree(&pattern_node) - loop_degree;
        let out_degree = pattern.out_degree(&pattern_node) - loop_degree;

        warn!(
            "\t\t{:?} {} {} {}",
            pattern_node, in_degree, out_degree, loop_degree
        );
        let in_candidates = in_degree_mmap.get_all(&in_degree).collect_vec();
        let out_candidates = out_degree_mmap.get_all(&out_degree).collect_vec();
        let loop_candidates = loop_degree_mmap.get_all(&loop_degree).collect_vec();
        warn!("\t\tIN CAND:   {:?}", in_candidates);
        warn!("\t\tOUT CAND:  {:?}", out_candidates);
        warn!("\t\tLOOP CAND: {:?}", loop_candidates);

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
    warn!(
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
        warn!(
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
        warn!("\nBY DEGREES ({}): {:?}", candidates_found, candidates);

        if candidates_found == bindings.len() {
            results.push(HashMap::from_iter(
                bindings
                    .iter()
                    .map(|(k, v)| (*k, state.candidate_mapping.get(v).unwrap().1))
                    .collect_vec(),
            ));

            warn!("\tRESULTS FOUND: {:?}", bindings,);
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

    for start_node_in_pattern in pattern.get_objects() {
        let pid = start_node_in_pattern.id;
        let start_candidates_in_target = state
            .candidates
            .get_all(&start_node_in_pattern.id)
            .collect_vec();

        for end_node_in_pattern in pattern.get_forward_neighbors(&start_node_in_pattern) {
            let tid = end_node_in_pattern.id;
            let end_candidates_in_target = state
                .candidates
                .get_all(&end_node_in_pattern.id)
                .collect_vec();

            for &start_candidate_in_target in &start_candidates_in_target {
                for &end_candidate_in_target in &end_candidates_in_target {
                    if start_candidate_in_target == end_candidate_in_target {
                        continue;
                    }

                    if !reachability
                        .are_adjacent(*start_candidate_in_target, *end_candidate_in_target)
                    {
                        continue;
                    }

                    let cand1 = state
                        .rev_candidate_mapping
                        .get(&(pid, *start_candidate_in_target))
                        .unwrap();
                    let cand2 = state
                        .rev_candidate_mapping
                        .get(&(tid, *end_candidate_in_target))
                        .unwrap();

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
        let mut values = HashSet::new();
        for v in result.values() {
            values.insert(v);
        }

        if values.len() < result.len() {
            continue;
        }

        for (k, v) in result {
            let _ = mosaic.get(k).map(|k| {
                if k.get_component("PatternMatchElement").is_none() {
                    k.add_component("PatternMatchElement", par(match_process.0.id as u64));
                }
            });

            let _ = mosaic.get(v).map(|v| {
                if v.get_component("PatternMatchElement").is_none() {
                    v.add_component("PatternMatchElement", par(match_process.0.id as u64));
                }
            });
            let binding_pair = mosaic.make_pair(&k, &v);
            bindings.add_back(&binding_pair);
        }

        match_process.add_result(&bindings);
    }

    transient.into_iter().delete();
    mosaic
        .get_all()
        .include_component("PatternMatchBinding")
        .delete();
    mosaic
        .get_all()
        .include_component("PatternMatchCandidate")
        .delete();
    Ok(match_process.0.clone())
}

#[cfg(test)]
mod pattern_match_tests {
    use log::warn;
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
                warn!("{:?} -> {:?}", bind.get_first(), bind.get_second());
            }
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
                warn!("{:?} -> {:?}", bind.get_first(), bind.get_second());
            }
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
                warn!("{:?} -> {:?}", bind.get_first(), bind.get_second());
            }
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
                warn!("{:?} -> {:?}", bind.get_first(), bind.get_second());
            }
        }
    }
}

pub fn pattern_match_tool(
    window: &GraspEditorWindow,
    ui: &GuiState,
    _initial_state: &[Tile],
    _tile: &Tile,
) -> TransformerState {
    ui.window("Pattern Match")
        .build(|| {
            let pick1 = window
                .document_mosaic
                .get_all()
                .include_component("Pick1")
                .next();

            if let Some(p1) = &pick1 {
                ui.text(format!("Pattern (pick #1): {}", p1.id));
                ui.same_line();
                let p = ColorQuery(&p1.target()).query();
                ui.color_button("Pick 1", [p.x, p.y, p.z, p.w]);
            } else {
                ui.text("Pattern (pick #1): Nothing");
            }
            ui.spacing();
            let pick2 = window
                .document_mosaic
                .get_all()
                .include_component("Pick2")
                .next();
            if let Some(p2) = &pick2 {
                ui.text(format!("Target (pick #2): {}", p2.id));
                ui.same_line();
                let p = ColorQuery(&p2.target()).query();
                ui.color_button("Pick 2", [p.x, p.y, p.z, p.w]);
            } else {
                ui.text("Target (pick #2): Nothing");
            }

            ui.separator();
            let token = if pick1.is_none() || pick2.is_none() {
                Some(ui.begin_disabled(true))
            } else {
                None
            };

            if ui.button_with_size("Run", [100.0, 20.0]) {
                let p = window.document_mosaic.make_procedure("PatternMatch");
                p.add_argument("pattern", &pick1.as_ref().unwrap().target());
                p.add_argument("target", &pick2.as_ref().unwrap().target());

                match pattern_match(&p) {
                    Ok(_) => {
                        warn!("PATTERN MATCH OK!");
                        p.0.add_component("PatternMatch", void());
                        for result in p.get_results() {
                            let list = ListTile::from_tile(result).unwrap();
                            for binding in list.iter() {
                                let bind = PairTile::from_tile(binding).unwrap();
                                warn!("{:?} -> {:?}", bind.get_first(), bind.get_second());
                            }
                        }

                        pick1.unwrap().iter().delete();
                        pick2.unwrap().iter().delete();
                    }
                    Err(e) => {
                        warn!("PATTERN MATCH ERROR: {:?}!", e.to_string());
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

pub fn pattern_match_property_renderer(s: &GuiState, window: &mut GraspEditorWindow, input: Tile) {
    let proc = ProcedureTile(input.target());

    let args = proc.get_arguments();
    let mut ok = args.is_some();

    for res in proc.get_results() {
        if let Some(binding_list) = ListTile::from_tile(res.clone()) {
            for binding in binding_list.iter() {
                if let Some(pair) = PairTile::from_tile(binding) {
                    // println!("1: {:?}", pair.get_first());
                    // println!("2: {:?}", pair.get_second());
                    if pair.get_first().is_none() || pair.get_second().is_none() {
                        ok = false;
                        break;
                    }
                } else {
                    ok = false;
                    break;
                }
            }
        } else {
            ok = false;
            break;
        }
    }

    if ok {
        procedure_args_renderer(s, window, input.target());
        pattern_match_result_renderer(s, window, proc);
    } else {
        window.delete_tiles(&[proc.0]);
    }
}

pub fn on_pattern_match_element_deleted(window: &mut GraspEditorWindow, _comp: String, pm: &Tile) {
    if let Some(p) = pm.mosaic.get(pm.get("self").as_u64() as usize) {
        let proc = ProcedureTile(p);

        for res in proc.get_results() {
            if let Some(binding_list) = ListTile::from_tile(res.clone()) {
                for binding in binding_list.iter() {
                    if let Some(pair) = PairTile::from_tile(binding) {
                        let _ = pair
                            .get_first()
                            .and_then(|m| m.get_component("PairElement"))
                            .map(|m| m.iter().delete());
                        let _ = pair
                            .get_second()
                            .and_then(|m| m.get_component("PairElement"))
                            .map(|m| m.iter().delete());
                        window.delete_tiles(&[pair.0]);
                    }
                }
            }
            window.delete_tiles(&[res]);
        }
        window.delete_tiles(&[proc.0]);
        pm.mosaic
            .get_all()
            .include_component("PatternMatchElement")
            .delete();
    }
}

pub fn on_pattern_match_deleted(window: &mut GraspEditorWindow, comp: String, pm: &Tile) {
    assert_eq!(&comp, "PatternMatch");

    let proc = ProcedureTile(pm.target());
    for res in proc.get_results() {
        if let Some(binding_list) = ListTile::from_tile(res.clone()) {
            for binding in binding_list.iter() {
                if let Some(pair) = PairTile::from_tile(binding) {
                    window.delete_tiles(&[pair.0]);
                }
            }
        }
        window.delete_tiles(&[res]);
    }
}

pub fn pattern_match_result_renderer(
    s: &GuiState,
    window: &mut GraspEditorWindow,
    proc: ProcedureTile,
) {
    if s.ui
        .collapsing_header("Results", TreeNodeFlags::DEFAULT_OPEN)
    {
        if s.ui.button_with_size("Clear Display", [150.0, 20.0]) {
            window
                .document_mosaic
                .get_all()
                .include_component("PatternMatchShow")
                .delete();
        }

        for (i, result) in proc.get_results().iter().enumerate() {
            if s.ui
                .button_with_size(format!("Result {}", i), [150.0, 20.0])
            {
                window
                    .document_mosaic
                    .get_all()
                    .include_component("PatternMatchShow")
                    .delete();

                window.document_mosaic.new_object(
                    "PatternMatchShow",
                    pars().set("choice", result.id as u64).ok(),
                );
            }
        }
    }
}

pub fn pattern_match_renderer_laser(
    _s: &GuiState,
    window: &mut GraspEditorWindow,
    input: Tile,
    painter: &mut DrawListMut<'_>,
) {
    if let Some(chosen_result) = window
        .document_mosaic
        .get_all()
        .include_component("PatternMatchShow")
        .next()
    {
        let id = chosen_result.get("choice").as_u64() as usize;
        if let Some(show) = window.document_mosaic.get(id) {
            if let Some(list) = ListTile::from_tile(show) {
                let mut bindings = list
                    .iter()
                    .flat_map(|binding| {
                        if let Some(pair) = PairTile::from_tile(binding) {
                            let fst = pair.get_first();
                            let snd = pair.get_second();
                            if let (Some(fst), Some(snd)) = (fst, snd) {
                                Some((fst, snd))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect_vec();

                bindings.sort_by_key(|a| a.0.clone());

                let mut index = 1;

                let mut min_x = 10000.0;
                let mut min_y = 10000.0;
                let mut max_x = -10000.0;
                let mut max_y = -10000.0;

                for (fst, snd) in &bindings {
                    let fst = window.get_position_with_offset_and_pan(PosQuery(fst).query());
                    let snd = window.get_position_with_offset_and_pan(PosQuery(snd).query());

                    painter.add_text(
                        [fst.x + 15.0, fst.y - 10.0],
                        ImColor32::WHITE,
                        format!("{}", index),
                    );

                    painter.add_text(
                        [snd.x + 15.0, snd.y - 10.0],
                        ImColor32::WHITE,
                        format!("{}", index),
                    );

                    let pos = snd;
                    if pos.x < min_x {
                        min_x = pos.x;
                    }
                    if pos.y < min_y {
                        min_y = pos.y;
                    }
                    if pos.x > max_x {
                        max_x = pos.x;
                    }
                    if pos.y > max_y {
                        max_y = pos.y;
                    }

                    painter
                        .add_line(
                            [fst.x, fst.y],
                            [snd.x, snd.y],
                            ImColor32::from_rgba_f32s(0.5, 0.5, 1.0, 0.2),
                        )
                        .thickness(10.0)
                        .build();

                    let is_selected = window.editor_data.selected.contains(&chosen_result);
                    let image = if is_selected { "[dot]" } else { "dot" };

                    gui_draw_image(
                        image,
                        [20.0, 20.0],
                        [snd.x - window.rect.x, snd.y - window.rect.y],
                        0.0,
                        1.0,
                        None,
                    );

                    index += 1;
                }

                let min_xy = Vec2::new(min_x, min_y);
                let max_xy = Vec2::new(max_x, max_y);

                let c1 = ImColor32::from_rgba(45, 45, 45, 128);
                let c2 = ImColor32::from_rgba(45, 40, 45, 128);
                let c3 = ImColor32::from_rgba(40, 45, 40, 128);
                let c4 = ImColor32::from_rgba(40, 40, 40, 128);

                painter.add_rect_filled_multicolor(
                    [min_xy.x - 60.0, min_xy.y - 60.0],
                    [max_xy.x + 60.0, max_xy.y + 60.0],
                    c1,
                    c2,
                    c3,
                    c4,
                );
            }
        }
    }
}

pub fn pattern_match_renderer(
    _s: &GuiState,
    window: &mut GraspEditorWindow,
    _input: Tile,
    painter: &mut DrawListMut<'_>,
) {
    let mosaic = &window.document_mosaic;

    if let Some(chosen_result) = mosaic
        .get_all()
        .include_component("PatternMatchShow")
        .next()
    {
        painter
            .add_rect(
                [0.0, 0.0],
                [window.rect.max().x, window.rect.max().y],
                ImColor32::from_rgba_f32s(0.0, 0.0, 0.0, 0.65),
            )
            .filled(true)
            .build();

        let id = chosen_result.get("choice").as_u64() as usize;
        if let Some(show) = window.document_mosaic.get(id) {
            if let Some(list) = ListTile::from_tile(show) {
                let mut bindings = list
                    .iter()
                    .flat_map(|binding| {
                        if let Some(pair) = PairTile::from_tile(binding) {
                            let fst = pair.get_first();
                            let snd = pair.get_second();
                            if let (Some(fst), Some(snd)) = (fst, snd) {
                                Some((fst, snd))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect_vec();

                bindings.sort_by_key(|a| a.0.clone());

                let bindings_map: HashMap<Tile, Tile> = HashMap::from_iter(bindings.clone());

                for (node, target) in &bindings {
                    let mut done = HashSet::new();
                    for arrow in node.iter().get_arrows_from() {
                        if done.contains(&arrow.target_id()) {
                            continue;
                        }

                        draw_arrow(window, painter, &arrow, 2.0);
                        done.insert(arrow.target_id());

                        if let Some(target_node2) = bindings_map.get(&arrow.target()) {
                            if let Some(arrow) = target
                                .iter()
                                .get_arrows_from()
                                .filter(|t| &t.target() == target_node2)
                                .sorted_by_key(|a| a.id)
                                .next()
                            {
                                draw_arrow(window, painter, &arrow, 4.0);
                            }
                        }
                    }
                }

                for (index, (node, target)) in bindings_map
                    .iter()
                    .sorted_by_key(|t| (PosQuery(t.0).query().y * 100.0) as u32)
                    .enumerate()
                {
                    for t in &[node, target] {
                        let pos = window.get_position_with_offset_and_pan(PosQuery(t).query());
                        painter
                            .add_rect(
                                [pos.x, pos.y - 10.0],
                                [pos.x + 40.0, pos.y + 10.0],
                                ImColor32::BLACK,
                            )
                            .filled(true)
                            .build();
                        painter
                            .add_rect(
                                [pos.x, pos.y - 10.0],
                                [pos.x + 40.0, pos.y + 10.0],
                                ImColor32::WHITE,
                            )
                            .filled(false)
                            .build();

                        painter.add_text(
                            [pos.x + 20.0, pos.y - 8.0],
                            ImColor32::WHITE,
                            format!("{}", index),
                        );
                    }
                }

                for (key, value) in bindings_map {
                    let pos = window.get_position_with_offset_and_pan(PosQuery(&key).query());
                    draw_node(&key, pos, window, painter);
                    let pos = window.get_position_with_offset_and_pan(PosQuery(&value).query());
                    draw_node(&value, pos, window, painter);
                }
            }
        }
    }
}
