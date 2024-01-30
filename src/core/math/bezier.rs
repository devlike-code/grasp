use std::f32::consts;

use imgui::{
    sys::{igImBezierQuadraticCalc, ImVec2},
    DrawListMut, ImColor32,
};

use crate::{core::gui::windowing::gui_draw_image, grasp_render::angle_between_points};

use super::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct BezierArrowHead {
    pub length: f32,
    pub width: f32,
    pub direction: Option<Vec2>,
}

pub fn gui_linear_bezier_dt(p0: Vec2, p1: Vec2) -> Vec2 {
    p1 - p0
}

pub fn gui_quad_bezier_dt(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    2.0 * (1.0 - t) * (p1 - p0) + 2.0 * t * (p2 - p1)
}

pub fn gui_cubic_bezier_dt(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let a = 1.0 - t;
    let b = a * a;
    let c = t * t;
    let d = 2.0 * t * a;

    -3.0 * p0 * b + 3.0 * p1 * (b - d) + 3.0 * p2 * (d - c) + 3.0 * p3 * c
}

pub fn gui_bezier_tangent(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let cp0_zero = (p1 - p0).len_sqr() < f32::EPSILON;
    let cp1_zero = (p3 - p2).len_sqr() < f32::EPSILON;

    match (cp0_zero, cp1_zero) {
        (true, true) => gui_linear_bezier_dt(p0, p3),
        (true, false) => gui_quad_bezier_dt(p0, p2, p3, t),
        (false, true) => gui_quad_bezier_dt(p0, p1, p3, t),
        _ => gui_cubic_bezier_dt(p0, p1, p2, p3, t),
    }
}

fn gui_bezier_get_point(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let mut v: ImVec2 = ImVec2 { x: 0.0, y: 0.0 };
    unsafe {
        igImBezierQuadraticCalc(
            &mut v as *mut ImVec2,
            [p0.x, p0.y].into(),
            [p1.x, p1.y].into(),
            [p2.x, p2.y].into(),
            t,
        );
    }
    Vec2::new(v.x, v.y)
}

fn gui_bezier_control_point(p0: Vec2, b: Vec2, p2: Vec2) -> Vec2 {
    let p = 2.0 * b - 0.5 * p0 - 0.5 * p2;
    assert_eq!(b, gui_bezier_get_point(p0, p, p2, 0.5));

    p
}

pub fn gui_draw_bezier_arrow(
    draw_list: &mut DrawListMut<'_>,
    points: [Vec2; 3],
    thickness: f32,
    quality: u32,
    window_pos: Vec2,
    offset: f32,
    color: ImColor32,
) {
    gui_draw_bezier_with_end_arrow(
        draw_list,
        [points[0], points[1], points[2]],
        quality,
        thickness,
        window_pos,
        offset,
        color,
    );
}

pub fn gui_draw_bezier(
    draw_list: &mut DrawListMut<'_>,
    points: [Vec2; 3],
    thickness: f32,
    quality: u32,
) {
    let ctrlp = gui_bezier_control_point(points[0], points[1], points[2]);

    let mut ps = vec![];
    let dq = 1.0 / quality as f32;
    for i in 0..=quality {
        let p = gui_bezier_get_point(points[0], ctrlp, points[2], dq * i as f32);
        ps.push([p.x, p.y]);
    }

    draw_list
        .add_polyline(ps, ImColor32::WHITE)
        .thickness(thickness)
        .filled(false)
        .build();
}

pub fn gui_draw_bezier_with_end_arrow(
    draw_list: &mut DrawListMut<'_>,
    points: [Vec2; 3],
    quality: u32,
    thickness: f32,
    window_pos: Vec2,
    offset: f32,
    color: ImColor32,
) {
    let ctrlp = gui_bezier_control_point(points[0], points[1], points[2]);

    let mut ps = vec![];
    let dq = 1.0 / quality as f32;
    for i in 0..=quality {
        let p = gui_bezier_get_point(points[0], ctrlp, points[2], dq * i as f32);
        ps.push([p.x, p.y]);
    }

    let end_dir = gui_bezier_tangent(points[0], ctrlp, ctrlp, points[2], 1.0).normalized();
    let angle = angle_between_points(end_dir, Vec2::ZERO) - consts::PI * 0.5;
    let tip = points[2] - end_dir * offset;

    gui_draw_image(
        "arrowhead",
        [20.0, 20.0],
        [tip.x - window_pos.x, tip.y - window_pos.y],
        angle,
        1.0,
        None,
    );

    draw_list
        .add_polyline(ps, color)
        .thickness(thickness)
        .filled(false)
        .build();
}
