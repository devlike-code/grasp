use imgui::{
    sys::{igImBezierQuadraticCalc, ImVec2},
    DrawListMut, ImColor32,
};
use itertools::Itertools;
use quadtree_rs::point;

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

/*
    For P0 and P2 as endpoints, and P1 as a control point, a point ON the bezier curve at some percent t is given by:

        B(t) = (1 - t)^2 P0 + 2(1 - t)t P1 + t^2 P2.

    In our case, t = 0.5, so we can simplify the above to:

        B = (1 - 0.5)^2 P0 + 2(1 - 0.5) 0.5 P1 + 0.5^2 P2
        B = 0.5^2 P0 + 0.5 P1 + 0.5^2 P2.

    In practice, we have P0, P2 and B (our node point on the curve!), so we want to solve for P1:

        B = 0.5^2 P0 + 0.5 P1 + 0.5^2 P2
        B = 0.5 (0.5 P0 + P1 + 0.5 P2)
        B = 0.5 (0.5 (P0 + P2) + P1)

    Switching to 1/2 form for ease of use:

        2 B = (P0 + P2) / 2 + P1
        P1 = 2 B - (P0 + P2) / 2.

    In practice, (P0 + P2) / 2 is the midpoint between the ends M, so:

        P1 = 2 B - M

    P1 is now the control point needed to draw the bezier curve that passes through B.
*/

// i was certain this would fail, but... uhm no. so i'm not sure why our point isn't on the line...
fn gui_bezier_control_point(p0: Vec2, b: Vec2, p2: Vec2) -> Vec2 {
    let p = 2.0 * b - 0.5 * p0 - 0.5 * p2;

    unsafe {
        let mut v: ImVec2 = ImVec2 { x: 0.0, y: 0.0 };
        igImBezierQuadraticCalc(
            &mut v as *mut ImVec2,
            [p0.x, p0.y].into(),
            [p.x, p.y].into(),
            [p2.x, p2.y].into(),
            0.5,
        );
        assert_eq!([v.x, v.y], [b.x, b.y]);
    }

    p
}

pub fn gui_draw_bezier_arrow(
    draw_list: &mut DrawListMut<'_>,
    points: [Vec2; 3],
    thickness: f32,
    color: ImColor32,
) {
    // let mid = points[0].lerp(points[2], 0.5);
    // let l = (points[1] - mid) * 0.5;

    let ctrlp = gui_bezier_control_point(points[0], points[1], points[2]);
    gui_draw_bezier_with_arrows(
        draw_list,
        [points[0], ctrlp, ctrlp, points[2]],
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

    draw_list
        .add_circle([points[0].x, points[0].y], 5.0, ImColor32::WHITE)
        .build();

    draw_list
        .add_circle([points[1].x, points[1].y], 5.0, ImColor32::WHITE)
        .build();

    draw_list
        .add_circle([ctrlp.x, ctrlp.y], 5.0, ImColor32::WHITE)
        .build();
}

pub fn gui_draw_bezier_with_arrows(
    draw_list: &mut DrawListMut<'_>,
    points: [Vec2; 4],
    thickness: f32,
    color: ImColor32,
    start_arrow: BezierArrowHead,
    end_arrow: BezierArrowHead,
) {
    let half_thickness = thickness * 0.5;

    if start_arrow.length > 0.0 {
        let start_dir =
            gui_bezier_tangent(points[0], points[1], points[2], points[3], 0.0).normalized();

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
        let end_dir =
            gui_bezier_tangent(points[0], points[1], points[2], points[3], 1.0).normalized();

        let end_n = Vec2::new(-end_dir.y, end_dir.x);
        let half_width = end_arrow.width * 0.5;
        let tip = points[3] - 1.0 * end_dir * end_arrow.length;

        let mut polyline: Vec<[f32; 2]> = vec![];
        let p3: Vec2 = points[3];
        if half_width > half_thickness {
            polyline.push((p3 - 2.0 * end_dir * end_arrow.length + end_n * half_width).into());
        }
        polyline.push(tip.into());
        if half_width > half_thickness {
            polyline.push((p3 - 2.0 * end_dir * end_arrow.length - end_n * half_width).into());
        }
        draw_list.add_polyline(polyline, color).filled(true).build();
    }

    let ps: Vec<[f32; 2]> = points.iter().map(|p| (*p).into()).collect_vec();
    draw_list
        .add_bezier_curve(ps[0], ps[1], ps[2], ps[3], color)
        .thickness(thickness)
        .build();
}
//     {
//         if (startArrowSize > 0.0f)
//         {
//             const auto start_dir  = ImNormalized(ImCubicBezierTangent(curve.P0, curve.P1, curve.P2, curve.P3, 0.0f));
//             const auto start_n    = ImVec2(-start_dir.y, start_dir.x);
//             const auto half_width = startArrowWidth * 0.5f;
//             const auto tip        = curve.P0 - start_dir * startArrowSize;

//             if (half_width > half_thickness)
//                 drawList->PathLineTo(curve.P0 - start_n * half_width);
//             drawList->PathLineTo(tip);
//             if (half_width > half_thickness)
//                 drawList->PathLineTo(curve.P0 + start_n * half_width);
//         }

//         ImDrawList_PathBezierOffset(drawList, half_thickness, curve.P0, curve.P1, curve.P2, curve.P3);

//         if (endArrowSize > 0.0f)
//         {
//             const auto    end_dir = ImNormalized(ImCubicBezierTangent(curve.P0, curve.P1, curve.P2, curve.P3, 1.0f));
//             const auto    end_n   = ImVec2(  -end_dir.y,   end_dir.x);
//             const auto half_width = endArrowWidth * 0.5f;
//             const auto tip        = curve.P3 + end_dir * endArrowSize;

//             if (half_width > half_thickness)
//                 drawList->PathLineTo(curve.P3 + end_n * half_width);
//             drawList->PathLineTo(tip);
//             if (half_width > half_thickness)
//                 drawList->PathLineTo(curve.P3 - end_n * half_width);
//         }

//         ImDrawList_PathBezierOffset(drawList, half_thickness, curve.P3, curve.P2, curve.P1, curve.P0);

//         drawList->PathStroke(color, true, strokeThickness);
//     }
// }
