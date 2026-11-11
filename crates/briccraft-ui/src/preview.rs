//! Small part illustrations are native shapes, with real studs and molded edges.
use crate::PartCard;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::Geometry;
use ferroui_view::prelude::*;

pub fn brick(part: &PartCard, color: &str, scale: f64) -> Element {
    let name = part.name.to_lowercase();
    let width = (part.size[0].max(1) as f64 * 8.0).clamp(16.0, 50.0) * scale;
    let depth = (part.size[2].max(1) as f64 * 6.0).clamp(12.0, 28.0) * scale;
    let illustration: Element =
        if name.contains("wheel") || name.contains("tire") || name.contains("gear") {
            let gear = name.contains("gear");
            grid()
                .width(36.0 * scale)
                .height(36.0 * scale)
                .children((
                    ellipse()
                        .fill(if gear { "#B0BCC3" } else { "#141A1D" })
                        .stroke("#52636C")
                        .stroke_thickness(2.0),
                    ellipse()
                        .fill(if gear { "#253640" } else { "#C4CFD4" })
                        .width(17.0 * scale)
                        .height(17.0 * scale)
                        .horizontal_alignment(H::Center)
                        .vertical_alignment(V::Center),
                    text_block()
                        .text("+")
                        .font_size(19.0 * scale)
                        .foreground("#30444F")
                        .horizontal_alignment(H::Center)
                        .vertical_alignment(V::Center),
                ))
                .into()
        } else if name.contains("slope") || name.contains("wedge") || name.contains("wing") {
            path()
                .data(Geometry::parse("M 0 28 L 43 2 L 43 28 Z").ok())
                .fill(color)
                .stroke("#55FFFFFF")
                .stroke_thickness(1.0)
                .width(45.0 * scale)
                .height(30.0 * scale)
                .into()
        } else if name.contains("window") || name.contains("windshield") || name.contains("door") {
            border()
                .width(36.0 * scale)
                .height(29.0 * scale)
                .corner_radius(3.0)
                .border_thickness(4.0 * scale)
                .border_brush(color)
                .background("#885DDAE1")
                .child(
                    grid().children((
                        border()
                            .width(3.0 * scale)
                            .background(color)
                            .horizontal_alignment(H::Center),
                        border()
                            .height(3.0 * scale)
                            .background(color)
                            .vertical_alignment(V::Center),
                    )),
                )
                .into()
        } else if name.contains("axle") || name.contains("rail") {
            border()
                .width(48.0 * scale)
                .height(7.0 * scale)
                .background(color)
                .border_brush("#90FFFFFF")
                .border_thickness((0.0, 1.0, 0.0, 1.0))
                .corner_radius(2.0)
                .into()
        } else {
            let columns = part.size[0].clamp(1, 6) as usize;
            let rows = part.size[2].clamp(1, 3) as usize;
            let mut studs = Vec::new();
            for row in 0..rows {
                for col in 0..columns {
                    let stud: Element = ellipse()
                        .width(5.2 * scale)
                        .height(3.8 * scale)
                        .fill(color)
                        .stroke("#66FFFFFF")
                        .stroke_thickness(0.8)
                        .canvas_left(
                            4.0 * scale
                                + col as f64 * (width - 9.0 * scale) / (columns.max(2) - 1) as f64,
                        )
                        .canvas_top(
                            3.0 * scale
                                + row as f64 * (depth - 7.0 * scale) / (rows.max(2) - 1) as f64,
                        )
                        .into();
                    studs.push(stud);
                }
            }
            grid()
                .width(width + 2.0)
                .height(depth + 6.0 * scale)
                .children((
                    border()
                        .background(color)
                        .corner_radius(2.5)
                        .margin((0.0, 5.0 * scale, 0.0, 0.0))
                        .border_brush("#77000000")
                        .border_thickness((0.0, 0.0, 0.0, 4.0 * scale)),
                    border()
                        .height(depth)
                        .vertical_alignment(V::Top)
                        .background(color)
                        .corner_radius(2.5)
                        .border_brush("#60FFFFFF")
                        .border_thickness((1.0, 1.0, 1.0, 0.0))
                        .child(canvas().children(studs)),
                ))
                .into()
        };
    grid()
        .width(62.0 * scale)
        .height(42.0 * scale)
        .is_hit_test_visible(false)
        .child(
            border()
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center)
                .child(illustration),
        )
        .into()
}
