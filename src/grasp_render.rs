use crate::core::gui::windowing::gui_draw_image;
use crate::core::math::bezier::gui_draw_bezier_arrow;
use crate::editor_state_machine::EditorState;
use crate::editor_state_machine::EditorStateTrigger::*;
use crate::editor_state_machine::StateMachine;
use crate::grasp_editor_window::GraspEditorWindow;

use crate::utilities::{Label, Pos};
use crate::GuiState;
use imgui::sys::ImVec2;
use imgui::DrawListMut;
use imgui::ImColor32;
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
    window: &mut GraspEditorWindow,
    painter: &DrawListMut<'_>,
    s: &GuiState,
) {
    let pos = window.get_position_with_pan(Pos(&tile).query());
    painter
        .add_circle([pos.x, pos.y], 10.0, ImColor32::from_rgb(255, 0, 0))
        .build();

    let image = if window.editor_data.selected.contains(&tile) {
        "[dot]"
    } else {
        "dot"
    };

    gui_draw_image(
        image,
        [20.0, 20.0],
        [pos.x - window.rect.x, pos.y - window.rect.y],
    );

    let mut cancel: bool = false;
   
                      
    if window.state == EditorState::PropertyChanging {
        if let Some(selected) = window.editor_data.selected.first() {
            if tile.id == selected.id {                
                let cx = pos.x - window.rect.x + 10.0;
                let cy = pos.y - window.rect.y;
                gui_set_cursor_pos(cx, cy);
                let text = &mut window.editor_data.text;

                s.ui.set_keyboard_focus_here();
                s.ui.set_next_item_width(100.0);
                if s.ui
                    .input_text(format!("##{}-EditorLabel", tile.id), text)
                    .auto_select_all(true)
                    .enter_returns_true(true)
                    .build()
                {
                    println!("enter pressed should save");
                    if text.len() >= 32 {
                        *text = text[0..32].to_string();
                    }

                    if let Ok(t) = text.parse::<String>() {
                        println!("text parsed should save");

                        if window.editor_data.previous_text != *text {
                            println!("new text should save {}", tile);
                            if let Some(mut label) = tile.clone().get_component("Label") {
                                label.set("self", t);
                            
                                window.trigger(EndDrag);
                            }
                        } else {
                            cancel = true;
                        }
                    } else {
                        cancel = true;
                    }
                }
            } else {
                cancel = true;
            }
        } else {
           cancel = true;
        }
    } else {
        cancel = true;
    }

    if cancel {
        let label = Label(tile).query();
        painter.add_text([pos.x + 10.0, pos.y], ImColor32::WHITE, label);
    }
}

pub fn default_renderer_draw(window: &mut GraspEditorWindow, s: &GuiState) {
    let mut painter = s.ui.get_window_draw_list();

    let arrows = window.document_mosaic.get_all().include_component("Arrow");

    for arrow in arrows {
        let p1 = window.get_position_with_pan(Pos(&arrow.source()).query());
        let p2 = window.get_position_with_pan(Pos(&arrow.target()).query());

        gui_draw_bezier_arrow(
            &mut painter,
            [p1, p1.lerp(p2, 0.5), p2],
            2.0,
            ImColor32::WHITE,
        );
    }

    let tiles = window
        .document_mosaic
        .get_all()
        .include_component("Position")
        .get_targets();

    if tiles.len() > 0 {
        for tile in tiles {
            if tile.is_object() {
                default_renderer_draw_object(&tile, window, &painter, s);
            }
        }
    }

    match window.state {
        EditorState::Link => {
            let a: [f32; 2] = window.editor_data.link_start_pos.unwrap().into();
            let pos: [f32; 2] = if let Some(b) = window.editor_data.link_end.as_ref() {
                window.get_position_with_pan(Pos(b).query()).into()
            } else {
                s.ui.io().mouse_pos
            };

            painter.add_line(a, pos, ImColor32::WHITE).build();
        }
        EditorState::Rect => {
            let a: [f32; 2] = {
                let position = window.editor_data.rect_start_pos.unwrap();
                window.get_position_with_pan(position).into()
            };

            let b: [f32; 2] = {
                let position = window.editor_data.rect_start_pos.unwrap()
                    + window.editor_data.rect_delta.unwrap();
                window.get_position_with_pan(position).into()
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
