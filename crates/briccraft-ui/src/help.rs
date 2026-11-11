//! A native, scrollable guide covers the controls available on each device.
use crate::{controls, theme, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn panel(state: UiState) -> Element {
    let keys: Vec<Element> = [
        ("DRAG", "Orbit around your construction"),
        ("SHIFT", "Hold while dragging to pan"),
        ("WHEEL", "Zoom toward or away from the world"),
        ("LMB", "Place the selected brick"),
        ("RMB", "Remove the targeted brick"),
        ("R", "Rotate the part by a quarter turn"),
        ("1–8", "Select a hotbar part"),
        ("B", "Open the library; brake while driving"),
        ("O", "Gameplay, resources and objectives"),
        ("TAB", "Cycle Build, Fly and Walk"),
        ("WASD", "Move, drive, or steer"),
        ("SPACE", "Jump in Walk; ascend in Fly"),
        ("Q / E", "Descend / ascend while flying"),
        ("F", "Activate / enter the nearest vehicle"),
        ("HOME", "Return to the overview"),
        ("CTRL Z", "Undo the last construction edit"),
        ("CTRL Y", "Redo an edit"),
        ("CTRL S", "Save the world in your browser"),
        ("ESC", "Close panels, leave vehicle, or pause"),
    ]
    .into_iter()
    .map(|(key, description)| {
        grid()
            .column_definitions("68,*")
            .margin((0.0, 0.0, 0.0, 9.0))
            .children((
                border()
                    .horizontal_alignment(H::Left)
                    .vertical_alignment(V::Top)
                    .child(controls::keycap(key)),
                text_block()
                    .grid_column(1)
                    .text(description)
                    .font_size(11.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
            ))
            .into_element()
    })
    .collect();
    let content = scroll_viewer().horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(12.0).children((
            controls::caption("Start in builder mode. Click a surface to snap a brick into place. Drag to look around, and use the wheel to get closer."),
            controls::caption("Open Plans to place starter constructions or save your own in My designs. Saved designs can be exported and imported between worlds."),
            separator(), stack_panel().children(keys), separator(),
            text_block().text("Connected by design").font_size(16.0).font_weight(FontWeight::SemiBold),
            controls::caption("Horizontal positions follow the stud grid. Vertical positions use plate units: three plates make one full brick. Axles, pins, gears and hinges provide mechanical connections."),
            controls::caption("On a touch screen, tap a surface to aim. Drag to orbit; move two fingers together to pan, and pinch to zoom. Place brick and Remove edit the target explicitly. Hold the movement pad with one thumb while looking around with the other. Brake and Park & exit appear while driving."),
        ))).into();
    border()
        .is_visible(state.help_open)
        .child(controls::panel(
            "How to play",
            "SMALL PIECES. BIG IDEAS.",
            content,
            move || state.help_open.set(false),
            state,
        ))
        .into()
}

fn separator() -> Element {
    border()
        .height(1.0)
        .background(theme::EDGE)
        .margin((0.0, 3.0))
        .into()
}
