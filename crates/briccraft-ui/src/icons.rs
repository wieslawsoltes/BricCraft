//! Small, consistent native vectors keep controls crisp at every display scale.
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::Geometry;
use ferroui_view::prelude::*;

#[derive(Clone, Copy)]
pub enum Icon {
    World,
    Places,
    Explore,
    Plans,
    Bricks,
    Settings,
    Save,
    Undo,
    Redo,
    Home,
    Drive,
    Park,
    Goals,
    Close,
    Plus,
    Rotate,
    Remove,
    Help,
    Import,
    Export,
}

fn data(icon: Icon) -> &'static str {
    match icon {
        Icon::World => "M 3 5 L 9 3 L 15 5 L 21 3 L 21 19 L 15 21 L 9 19 L 3 21 Z M 9 3 L 9 19 M 15 5 L 15 21",
        Icon::Places => "M 5 20 L 5 11 L 10 11 L 10 20 M 10 20 L 10 5 L 17 5 L 17 20 M 17 20 L 17 10 L 21 10 L 21 20 M 2 20 L 23 20 M 12 8 L 15 8 M 12 11 L 15 11 M 12 14 L 15 14",
        Icon::Explore => "M 12 2 A 10 10 0 1 0 12 22 A 10 10 0 1 0 12 2 M 16 7 L 14 14 L 7 17 L 10 10 Z",
        Icon::Plans => "M 5 3 L 16 3 L 21 8 L 21 21 L 5 21 Z M 16 3 L 16 8 L 21 8 M 8 11 L 17 11 M 8 15 L 17 15 M 8 18 L 13 18",
        Icon::Bricks => "M 3 8 L 21 8 L 21 21 L 3 21 Z M 6 8 L 6 4 L 10 4 L 10 8 M 14 8 L 14 4 L 18 4 L 18 8 M 3 15 L 21 15 M 12 15 L 12 21",
        Icon::Settings => "M 4 5 L 20 5 M 4 12 L 20 12 M 4 19 L 20 19 M 8 2 L 8 8 M 16 9 L 16 15 M 10 16 L 10 22",
        Icon::Save => "M 4 3 L 18 3 L 21 6 L 21 21 L 3 21 L 3 3 Z M 7 3 L 7 9 L 16 9 L 16 3 M 7 21 L 7 14 L 17 14 L 17 21",
        Icon::Undo => "M 8 4 L 3 9 L 8 14 M 3 9 L 14 9 C 22 9 22 20 14 20 L 9 20",
        Icon::Redo => "M 16 4 L 21 9 L 16 14 M 21 9 L 10 9 C 2 9 2 20 10 20 L 15 20",
        Icon::Home => "M 2 11 L 12 3 L 22 11 M 5 9 L 5 21 L 19 21 L 19 9 M 10 21 L 10 14 L 14 14 L 14 21",
        Icon::Drive => "M 3 16 L 3 10 L 7 9 L 10 5 L 17 5 L 20 10 L 22 11 L 22 17 L 19 17 M 7 17 L 15 17 M 10 5 L 10 10 L 20 10 M 7 18 A 2 2 0 1 0 3 18 A 2 2 0 1 0 7 18 M 20 18 A 2 2 0 1 0 16 18 A 2 2 0 1 0 20 18",
        Icon::Park => "M 6 21 L 6 3 L 14 3 C 22 3 22 13 14 13 L 6 13",
        Icon::Goals => "M 12 2 A 10 10 0 1 0 12 22 A 10 10 0 1 0 12 2 M 12 6 A 6 6 0 1 0 12 18 A 6 6 0 1 0 12 6 M 12 10 A 2 2 0 1 0 12 14 A 2 2 0 1 0 12 10",
        Icon::Close => "M 6 6 L 18 18 M 18 6 L 6 18",
        Icon::Plus => "M 12 4 L 12 20 M 4 12 L 20 12",
        Icon::Rotate => "M 4 10 A 8 8 0 1 1 5 18 M 4 4 L 4 10 L 10 10",
        Icon::Remove => "M 4 6 L 20 6 M 9 6 L 9 3 L 15 3 L 15 6 M 6 6 L 7 21 L 17 21 L 18 6 M 10 10 L 10 17 M 14 10 L 14 17",
        Icon::Import => "M 12 2 L 12 15 M 7 10 L 12 15 L 17 10 M 4 16 L 4 21 L 20 21 L 20 16",
        Icon::Export => "M 12 16 L 12 3 M 7 8 L 12 3 L 17 8 M 4 15 L 4 21 L 20 21 L 20 15",
        Icon::Help => "M 12 2 A 10 10 0 1 0 12 22 A 10 10 0 1 0 12 2 M 8 8 C 8 3 17 4 16 9 C 16 11 12 11 12 15 M 12 18 L 12 19",
    }
}

pub fn glyph(icon: Icon, color: &str) -> Element {
    path()
        .data(Geometry::parse(data(icon)).ok())
        .stroke(color)
        .stroke_thickness(1.7)
        .width(24.0)
        .height(24.0)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .is_hit_test_visible(false)
        .into()
}

pub fn label(icon: Icon, title: &str, color: &str) -> Element {
    stack_panel()
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .spacing(7.0)
        .is_hit_test_visible(false)
        .children((
            glyph(icon, color),
            text_block()
                .text(title)
                .foreground(color)
                .font_size(12.0)
                .vertical_alignment(V::Center),
        ))
        .into()
}
