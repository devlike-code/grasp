use std::f32::consts;

use crate::core::gui::windowing::gui_draw_image;
use crate::core::math::bezier::gui_draw_bezier_arrow;
use crate::core::math::Vec2;
use crate::editor_state_machine::EditorState;
use crate::editor_state_machine::EditorStateTrigger::*;
use crate::editor_state_machine::StateMachine;
use crate::grasp_editor_window::GraspEditorWindow;

use crate::grasp_transitions::query_position_recursive;
use crate::grasp_transitions::QuadtreeUpdateCapability;
use crate::utilities::Label;
use crate::utilities::Offset;
use crate::GuiState;
use imgui::sys::ImVec2;
use imgui::DrawListMut;
use imgui::ImColor32;
use itertools::Itertools;
use mosaic::capabilities::ArchetypeSubject;
use mosaic::internals::Tile;
use mosaic::internals::TileFieldEmptyQuery;
use mosaic::internals::{MosaicIO, TileFieldSetter};
use mosaic::iterators::component_selectors::ComponentSelectors;
use mosaic::iterators::tile_getters::TileGetters;

pub type GraspRenderer = fn(&mut GraspEditorWindow, &GuiState);

fn gui_set_cursor_pos(x: f32, y: f32) {
    unsafe {
        imgui::sys::igSetCursorPos(ImVec2::new(x, y));
    }
}

fn default_renderer_draw_object(
    tile: &Tile,
    pos: Vec2,
    window: &mut GraspEditorWindow,
    painter: &DrawListMut<'_>,
    s: &GuiState,
) {
    painter
        .add_circle([pos.x, pos.y], 10.0, ImColor32::from_rgb(255, 0, 0))
        .build();

    let is_selected = window.editor_data.selected.contains(tile);
    let image = if is_selected { "[dot]" } else { "dot" };

    gui_draw_image(
        image,
        [20.0, 20.0],
        [pos.x - window.rect.x, pos.y - window.rect.y],
        0.0,
        1.0,
    );

    let mut cancel: bool = true;
    let mut trigger_end_drag = true;
    let offset = tile
        .get_component("Label")
        .map(|l| Offset(&l).query())
        .unwrap_or_default();

    if window.state == EditorState::PropertyChanging
        && window.editor_data.tile_changing == Some(tile.id)
    {
        if let Some(selected) = window.editor_data.selected.first() {
            if tile.id == selected.id {
                let cx = pos.x - window.rect.x + offset.x;
                let cy = pos.y - window.rect.y + offset.y;
                gui_set_cursor_pos(cx, cy);
                let text = &mut window.editor_data.text;

                s.ui.set_keyboard_focus_here();
                s.ui.set_next_item_width(100.0);
                if s.ui
                    .input_text("##", text)
                    .auto_select_all(true)
                    .enter_returns_true(true)
                    .build()
                {
                    if text.len() >= 32 {
                        *text = text[0..32].to_string();
                    }

                    if let Ok(t) = text.parse::<String>() {
                        if window.editor_data.previous_text != *text {
                            if let Some(mut label) = tile.clone().get_component("Label") {
                                label.set("self", t);
                                window.document_mosaic.request_quadtree_update();
                            } else {
                                cancel = false;
                                trigger_end_drag = false;
                            }
                        }
                    }
                } else {
                    cancel = false;
                    trigger_end_drag = false;
                }
            }
        }
    } else {
        trigger_end_drag = false;
    }

    if trigger_end_drag {
        window.trigger(EndDrag);
    }

    if cancel {
        let label = Label(tile).query();
        painter.add_text(
            [pos.x + offset.x, pos.y + offset.y],
            ImColor32::WHITE,
            label,
        );
    }
}

pub fn angle_between_points(p1: Vec2, p2: Vec2) -> f32 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    let mut angle = dy.atan2(dx);
    if angle < 0.0 {
        angle += 2.0 * consts::PI;
    }

    angle + consts::PI
}

fn default_renderer_draw_arrow(
    tile: &Tile,
    pos: Vec2,
    window: &mut GraspEditorWindow,
    painter: &DrawListMut<'_>,
    s: &GuiState,
) {
    let is_selected = window.editor_data.selected.contains(tile);
    let image = if is_selected { "[arrow]" } else { "arrow" };

    let p = query_position_recursive(&tile.source());
    let q = query_position_recursive(&tile.target());
    let angle = angle_between_points(p, q);
    gui_draw_image(
        image,
        [20.0, 20.0],
        [pos.x - window.rect.x, pos.y - window.rect.y],
        angle,
        1.0,
    );

    let mut cancel: bool = true;
    let mut trigger_end_drag = true;
    let offset = tile
        .get_component("Label")
        .map(|l| Offset(&l).query())
        .unwrap_or_default();

    if window.state == EditorState::PropertyChanging
        && window.editor_data.tile_changing == Some(tile.id)
    {
        if let Some(selected) = window.editor_data.selected.first() {
            if tile.id == selected.id {
                let cx = pos.x - window.rect.x + offset.x;
                let cy = pos.y - window.rect.y + offset.y;
                gui_set_cursor_pos(cx, cy);
                let text = &mut window.editor_data.text;

                s.ui.set_keyboard_focus_here();
                s.ui.set_next_item_width(100.0);
                if s.ui
                    .input_text("##", text)
                    .auto_select_all(true)
                    .enter_returns_true(true)
                    .build()
                {
                    if text.len() >= 32 {
                        *text = text[0..32].to_string();
                    }

                    if let Ok(t) = text.parse::<String>() {
                        if window.editor_data.previous_text != *text {
                            if let Some(mut label) = tile.clone().get_component("Label") {
                                label.set("self", t);
                                window.document_mosaic.request_quadtree_update();
                            } else {
                                cancel = false;
                                trigger_end_drag = false;
                            }
                        }
                    }
                } else {
                    cancel = false;
                    trigger_end_drag = false;
                }
            }
        }
    } else {
        trigger_end_drag = false;
    }

    if trigger_end_drag {
        window.trigger(EndDrag);
    }

    if cancel {
        let label = Label(tile).query();
        painter.add_text(
            [pos.x + offset.x, pos.y + offset.y],
            ImColor32::WHITE,
            label,
        );
    }
}

pub fn default_renderer_draw(window: &mut GraspEditorWindow, s: &GuiState) {
    let mut painter = s.ui.get_window_draw_list();

    let arrows = window
        .document_mosaic
        .get_all()
        .include_component("Arrow")
        .collect_vec();

    for arrow in &arrows {
        let target = arrow.target();
        let p1 = window.get_position_with_offset_and_pan(query_position_recursive(&arrow.source()));
        let p2 = window.get_position_with_offset_and_pan(query_position_recursive(&target));

        let offset = Offset(arrow).query();
        let arrow_end_offset = if target.is_object() { 15.0f32 } else { 11.0f32 };

        let mid = p1.lerp(p2, 0.5) + offset;
        gui_draw_bezier_arrow(
            &mut painter,
            [p1, mid, p2],
            2.0,
            32,
            window.rect.min(),
            arrow_end_offset,
        );
    }

    for arrow in &arrows {
        let p1 = window.get_position_with_offset_and_pan(query_position_recursive(&arrow.source()));
        let p2 = window.get_position_with_offset_and_pan(query_position_recursive(&arrow.target()));
        let offset = Offset(arrow).query();

        let mid = p1.lerp(p2, 0.5) + offset;
        default_renderer_draw_arrow(arrow, mid, window, &painter, s);
    }

    let tiles = window
        .document_mosaic
        .get_all()
        .include_component("Position")
        .get_targets();

    if tiles.len() > 0 {
        for tile in tiles {
            if tile.is_object() {
                let pos = window.get_position_with_offset_and_pan(query_position_recursive(&tile));
                default_renderer_draw_object(&tile, pos, window, &painter, s);
            }
        }
    }

    match window.state {
        EditorState::Link => {
            let a: [f32; 2] = window.editor_data.link_start_pos.unwrap().into();
            let pos: [f32; 2] = if let Some(b) = window.editor_data.link_end.as_ref() {
                (window.get_position_with_offset_and_pan(query_position_recursive(b))).into()
            } else {
                s.ui.io().mouse_pos
            };
            //pos = window.get_position_with_offset_and_pan(pos.into()).into();

            painter.add_line(a, pos, ImColor32::WHITE).build();
        }
        EditorState::Rect => {
            let a: [f32; 2] = {
                let position = window.editor_data.rect_start_pos.unwrap();
                position.into()
                //window.get_position_with_offset_and_pan(position).into()
            };

            let b: [f32; 2] = {
                let position = window.editor_data.rect_start_pos.unwrap()
                    + window.editor_data.rect_delta.unwrap();
                position.into()
                // window.get_position_with_offset_and_pan(position).into()
            };

            painter.add_rect_filled_multicolor(
                a,
                b,
                ImColor32::from_rgba(77, 102, 128, 30),
                ImColor32::from_rgba(102, 77, 128, 30),
                ImColor32::from_rgba(77, 128, 102, 30),
                ImColor32::from_rgba(102, 128, 77, 30),
            );
            painter
                .add_rect(a, b, ImColor32::from_rgba(255, 255, 255, 40))
                .build();
        }
        _ => {}
    }
}
