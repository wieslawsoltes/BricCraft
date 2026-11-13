//! Native player location, compass and map controls, independent of the build pin.
use crate::{controls, icons, map_geometry as geo, theme, Dispatch, MapPlayer, UiAction, UiState};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{FontWeight, Geometry, TextWrapping},
};
use ferroui_view::prelude::*;

const LOCATION: &str = "#8CDAFF";

// Read visibility first so reactive dependency tracking drops the moving-player
// subscription while these native controls are hidden. Updating hidden text can
// otherwise invalidate layout even though the surrounding panel is not drawn.
fn places_player(state: UiState) -> Option<MapPlayer> {
    if state.places_open.get() {
        state.map_player.get()
    } else {
        None
    }
}

fn hud_player(state: UiState, compact: bool) -> Option<MapPlayer> {
    if state.compact.get() != compact || state.modal() || (compact && state.layout.get().keyboard) {
        None
    } else {
        state.map_player.get()
    }
}

pub fn marker(state: UiState) -> Element {
    rerender(move || {
        let Some(player) = places_player(state) else {
            return canvas().is_hit_test_visible(false).into_element();
        };
        let map = state.map_view.get();
        let size = geo::map_view_size(state.layout.get());
        let position = geo::screen_position(&map, size, player.latitude, player.longitude);
        let [tip, left, right] = player.arrow();
        let arrow = format!(
            "M {} {} L {} {} L {} {} Z",
            tip[0], tip[1], left[0], left[1], right[0], right[1]
        );
        canvas()
            .name("MapPlayerMarker")
            .is_hit_test_visible(false)
            .children((
                ellipse()
                    .width(48.0)
                    .height(48.0)
                    .canvas_left(position[0] - 24.0)
                    .canvas_top(position[1] - 24.0)
                    .fill("#306CCBFF")
                    .stroke("#708CDAFF")
                    .stroke_thickness(1.0),
                path()
                    .data(Geometry::parse(&arrow).ok())
                    .width(48.0)
                    .height(48.0)
                    .canvas_left(position[0] - 24.0)
                    .canvas_top(position[1] - 24.0)
                    .fill(LOCATION)
                    .stroke("#143246")
                    .stroke_thickness(2.0),
                ellipse()
                    .width(10.0)
                    .height(10.0)
                    .canvas_left(position[0] - 5.0)
                    .canvas_top(position[1] - 5.0)
                    .fill("#FFFFFF")
                    .stroke("#155E8A")
                    .stroke_thickness(2.0),
            ))
            .into_element()
    })
}

pub fn locate(state: UiState, dispatch: Dispatch) -> El<ferroui_controls::Button> {
    controls::tool_button(
        "MapLocatePlayer",
        "Locate your current position without changing the build selection",
        icons::Icon::Explore,
        move || dispatch(UiAction::LocateMapPlayer),
    )
    .is_visible(move || places_player(state).is_some())
    .background("#173C52")
}

/// Full geographic details live in the roomy Places panel, not a narrow phone chip.
pub fn details(state: UiState, dispatch: Dispatch) -> Element {
    border()
        .name("MapPlayerDetails")
        .is_visible(move || places_player(state).is_some())
        .background("#173546")
        .border_brush("#42647A")
        .border_thickness(1.0)
        .corner_radius(12.0)
        .padding(12.0)
        .child(stack_panel().spacing(7.0).children((
            controls::label("YOUR CURRENT LOCATION"),
            grid().column_definitions("*,Auto").children((
                text_block()
                    .name("MapPlayerHeading")
                    .text(move || places_player(state).map_or_else(String::new, |p| p.heading()))
                    .font_size(18.0)
                    .font_weight(FontWeight::SemiBold)
                    .foreground(LOCATION),
                text_block()
                    .grid_column(1)
                    .vertical_alignment(V::Center)
                    .text(move || {
                        places_player(state).map_or_else(String::new, |p| {
                            format!("{} from start", p.distance())
                        })
                    })
                    .font_size(11.0)
                    .foreground(theme::MUTED),
            )),
            text_block()
                .name("MapPlayerCoordinates")
                .text(move || places_player(state).map_or_else(String::new, |p| p.coordinates()))
                .font_size(12.0)
                .foreground("#DCECF8"),
            controls::caption("Blue arrow: your location and heading. Gold pin: the place selected for your next build."),
            button()
                .name("LocateCurrentPosition")
                .min_height(44.0)
                .content("Show my position on the map")
                .on_click(move |_| dispatch(UiAction::LocateMapPlayer)),
        )))
        .into()
}

pub fn summary(state: UiState) -> Element {
    stack_panel()
        .spacing(4.0)
        .is_visible(move || hud_player(state, false).is_some())
        .children((
            text_block()
                .text(move || {
                    hud_player(state, false).map_or_else(String::new, |p| {
                        format!("{}   ·   {} from start", p.heading(), p.distance())
                    })
                })
                .font_size(12.0)
                .font_weight(FontWeight::SemiBold)
                .foreground(LOCATION)
                .text_wrapping(TextWrapping::Wrap),
            text_block()
                .text(move || {
                    hud_player(state, false).map_or_else(String::new, |p| p.coordinates())
                })
                .font_size(11.0)
                .foreground(theme::MUTED),
        ))
        .into()
}

pub fn mobile(state: UiState, dispatch: Dispatch) -> Element {
    button()
        .name("MobileMapNavigation")
        .is_visible(move || hud_player(state, true).is_some())
        .grid_column(2)
        .margin((6.0, 0.0, 0.0, 0.0))
        .width(88.0)
        .height(48.0)
        .padding((5.0, 5.0))
        .background("#173546")
        .border_brush("#42647A")
        .horizontal_content_alignment(H::Center)
        .tool_tip_tip("Locate yourself on the map and view streaming status")
        .automation_properties_name(Some("Show my current map location".into()))
        .on_click(move |_| dispatch(UiAction::LocateMapPlayer))
        .child(
            stack_panel().spacing(3.0).children((
                text_block()
                    .text(move || hud_player(state, true).map_or_else(String::new, |p| p.heading()))
                    .font_size(11.0)
                    .font_weight(FontWeight::SemiBold)
                    .foreground(LOCATION),
                text_block()
                    .text("Locate on map")
                    .font_size(10.0)
                    .foreground(theme::MUTED),
            )),
        )
        .into()
}
