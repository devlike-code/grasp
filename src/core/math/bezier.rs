use imgui::{
    sys::{igImBezierQuadraticCalc, ImVec2},
    DrawListMut, ImColor32,
};

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

// i was certain this would fail, but... uhm no. so i'm not sure why our point isn't on the line...
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
    color: ImColor32,
) {
    gui_draw_bezier_with_arrows(
        draw_list,
        [points[0], points[1], points[2]],
        quality,
        thickness,
        color,
        BezierArrowHead {
            length: 0.0,
            width: 0.0,
            direction: None,
        },
        BezierArrowHead {
            length: 10.0,
            width: 10.0,
            direction: None,
        },
    );
}

pub fn gui_draw_bezier_with_arrows(
    draw_list: &mut DrawListMut<'_>,
    points: [Vec2; 3],
    quality: u32,
    thickness: f32,
    color: ImColor32,
    start_arrow: BezierArrowHead,
    end_arrow: BezierArrowHead,
) {
    let ctrlp = gui_bezier_control_point(points[0], points[1], points[2]);

    let mut ps = vec![];
    let dq = 1.0 / quality as f32;
    for i in 0..=quality {
        let p = gui_bezier_get_point(points[0], ctrlp, points[2], dq * i as f32);
        ps.push([p.x, p.y]);
    }

    let half_thickness = thickness * 0.5;

    if start_arrow.length > 0.0 {
        let start_dir = gui_bezier_tangent(points[0], ctrlp, ctrlp, points[2], 0.0).normalized();

        let start_n = Vec2::new(-start_dir.y, start_dir.x);
        let half_width = start_arrow.width * 0.5;
        let tip = points[0] - start_dir * start_arrow.length;

        let mut polyline: Vec<[f32; 2]> = vec![];
        let p0: Vec2 = points[0];
        if half_width > half_thickness {
            polyline.push((p0 - start_n * half_width).into());
        }
        polyline.push(tip.into());
        if half_width > half_thickness {
            polyline.push((p0 + start_n * half_width).into());
        }
        draw_list.add_polyline(polyline, color).filled(true).build();
    }

    if end_arrow.length > 0.0 {
        let end_dir = gui_bezier_tangent(points[0], ctrlp, ctrlp, points[2], 1.0).normalized();

        let end_n = Vec2::new(-end_dir.y, end_dir.x);
        let half_width = end_arrow.width * 0.5;
        let tip = points[2] - 1.0 * end_dir * end_arrow.length;

        let mut polyline: Vec<[f32; 2]> = vec![];
        let p3: Vec2 = points[2];
        if half_width > half_thickness {
            polyline.push((p3 - 2.0 * end_dir * end_arrow.length + end_n * half_width).into());
        }
        polyline.push(tip.into());
        if half_width > half_thickness {
            polyline.push((p3 - 2.0 * end_dir * end_arrow.length - end_n * half_width).into());
        }
        draw_list.add_polyline(polyline, color).filled(true).build();
    }

    draw_list
        .add_polyline(ps, ImColor32::WHITE)
        .thickness(thickness)
        .filled(false)
        .build();
}
