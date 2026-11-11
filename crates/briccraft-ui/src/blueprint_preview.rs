//! Distinct, resolution-independent construction illustrations for starter plans.
use ferroui_base::media::Geometry;
use ferroui_view::prelude::*;

fn shape(data: &str, color: &str) -> Element {
    path()
        .data(Geometry::parse(data).ok())
        .fill(color)
        .stroke("#B5D4E1ED")
        .stroke_thickness(1.1)
        .width(76.0)
        .height(62.0)
        .into()
}

fn line(data: &str, color: &str, thickness: f64) -> Element {
    path()
        .data(Geometry::parse(data).ok())
        .stroke(color)
        .stroke_thickness(thickness)
        .width(76.0)
        .height(62.0)
        .into()
}

fn wheel(x: f64) -> Element {
    ellipse()
        .width(15.0)
        .height(15.0)
        .canvas_left(x)
        .canvas_top(40.0)
        .fill("#101722")
        .stroke("#9CAFC1")
        .stroke_thickness(3.0)
        .into()
}

pub fn view(id: &str, color: &str) -> Element {
    let mut parts = vec![ellipse()
        .width(64.0)
        .height(8.0)
        .canvas_left(6.0)
        .canvas_top(53.0)
        .fill("#80101824")
        .into_element()];
    match id {
        "rover" => {
            parts.extend([
                shape("M 10 28 L 21 28 L 28 17 L 47 17 L 55 28 L 66 31 L 66 46 L 10 46 Z", color),
                shape("M 28 20 L 44 20 L 49 28 L 24 28 Z", "#8EC9D8"),
                wheel(13.0), wheel(49.0),
                line("M 11 33 L 64 33 M 36 29 L 36 43", "#9BEBF4F6", 1.2),
            ]);
        }
        "plane" => parts.extend([
            shape("M 8 32 L 30 28 L 41 8 L 47 9 L 44 27 L 67 21 L 70 26 L 43 37 L 38 55 L 32 54 L 33 40 L 10 42 Z", color),
            shape("M 36 27 L 45 24 L 45 29 L 37 33 Z", "#8EC9D8"),
            line("M 69 12 L 69 35 M 66 25 L 73 25", "#D4E5ED", 2.0),
        ]),
        "train" => {
            parts.extend([
                shape("M 12 19 L 38 19 L 38 30 L 63 30 L 68 38 L 68 47 L 12 47 Z", color),
                shape("M 17 23 L 32 23 L 32 32 L 17 32 Z", "#8EC9D8"),
                shape("M 49 29 L 49 16 L 58 16 L 58 29 Z", "#536B80"),
                wheel(14.0), wheel(34.0), wheel(54.0),
                line("M 18 48 L 62 48", "#EDF4F8", 2.0),
            ]);
        }
        "boat" => parts.extend([
            shape("M 9 42 L 67 42 L 57 54 L 21 54 Z", color),
            shape("M 35 9 L 35 39 L 12 39 Z", "#DCE8F1"),
            shape("M 40 14 L 60 38 L 40 38 Z", "#FFC36D"),
            line("M 38 8 L 38 46", "#B8D1DF", 2.0),
            line("M 9 59 Q 18 54 27 59 Q 36 64 45 59 Q 54 54 67 59", "#6EBBD3", 1.8),
        ]),
        "house" => parts.extend([
            shape("M 15 25 L 60 25 L 60 55 L 15 55 Z", "#D7CFB7"),
            shape("M 7 27 L 37 7 L 68 27 Z", color),
            shape("M 34 38 L 44 38 L 44 55 L 34 55 Z", "#334B61"),
            shape("M 21 33 L 29 33 L 29 42 L 21 42 Z M 49 33 L 55 33 L 55 42 L 49 42 Z", "#83C4D5"),
            line("M 15 30 L 60 30 M 15 47 L 32 47 M 46 47 L 60 47", "#807B7567", 1.0),
        ]),
        "tower" => parts.extend([
            shape("M 25 20 L 53 20 L 53 55 L 25 55 Z", color),
            shape("M 20 21 L 39 8 L 59 21 Z", "#E3A16D"),
            shape("M 34 28 L 44 28 L 44 35 L 34 35 Z M 34 44 L 44 44 L 44 55 L 34 55 Z", "#34556D"),
            line("M 25 39 L 53 39 M 39 3 L 39 9", "#DBE5EC", 1.5),
        ]),
        "bridge" => parts.extend([
            shape("M 7 34 L 69 34 L 69 55 L 59 55 C 59 36 44 36 44 55 L 33 55 C 33 36 18 36 18 55 L 7 55 Z", color),
            line("M 5 28 L 71 28 M 10 24 L 10 34 M 22 25 L 22 34 M 35 25 L 35 34 M 48 25 L 48 34 M 62 24 L 62 34", "#D7E1EA", 2.0),
            line("M 5 59 L 71 59", "#6EBBD3", 1.5),
        ]),
        _ => parts.extend([
            shape("M 29 27 L 48 27 L 53 55 L 24 55 Z", color),
            shape("M 25 28 L 38 16 L 52 28 Z", "#D4966D"),
            shape("M 34 43 L 43 43 L 43 55 L 34 55 Z", "#38516A"),
            line("M 38 24 L 18 8 M 38 24 L 59 8 M 38 24 L 18 43 M 38 24 L 59 43", "#DCEAF3", 4.0),
            ellipse().width(8.0).height(8.0).canvas_left(34.0).canvas_top(20.0)
                .fill("#FFC36D").stroke("#EAF2F7").stroke_thickness(1.5).into(),
        ]),
    }
    canvas()
        .width(76.0)
        .height(62.0)
        .is_hit_test_visible(false)
        .children(parts)
        .into()
}
