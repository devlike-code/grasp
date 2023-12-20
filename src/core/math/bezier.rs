use imgui::{DrawListMut, ImColor32};
use itertools::Itertools;

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
        draw_list.add_polyline(polyline, color).build();
    }

    if end_arrow.length > 0.0 {
        let end_dir =
            gui_bezier_tangent(points[0], points[1], points[2], points[3], 1.0).normalized();

        let end_n = Vec2::new(-end_dir.y, end_dir.x);
        let half_width = end_arrow.width * 0.5;
        // const auto tip = curve.P3 + end_dir * endArrowSize;
        let tip = points[3] + end_dir * end_arrow.length;

        let mut polyline: Vec<[f32; 2]> = vec![];
        let p3: Vec2 = points[3];
        if half_width > half_thickness {
            polyline.push((p3 + end_n * half_width).into());
        }
        polyline.push(tip.into());
        if half_width > half_thickness {
            polyline.push((p3 - end_n * half_width).into());
        }
        draw_list.add_polyline(polyline, color).build();
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
