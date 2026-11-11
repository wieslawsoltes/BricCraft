//! Geographic generation options, kept separate from model import and progress.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    let latitude = signal("52.516275".to_string());
    let longitude = signal("13.377704".to_string());
    let radius = signal("160".to_string());
    let scale = signal("4".to_string());
    let endpoint = signal("https://overpass-api.de/api/interpreter".to_string());
    let terrain = signal(true);
    let fit_height = signal(true);
    let exaggeration = signal(1.0);
    let advanced = signal(false);
    let error = signal(String::new());
    let local = dispatch.clone();
    let landmark = dispatch.clone();
    let sample = dispatch.clone();
    let generate = dispatch;
    let action = move |local: bool| -> Option<UiAction> {
        let values = [
            latitude.get_untracked(),
            longitude.get_untracked(),
            radius.get_untracked(),
            scale.get_untracked(),
        ];
        let parsed: Option<Vec<f64>> = values.iter().map(|s| s.trim().parse().ok()).collect();
        match parsed {
            Some(v)
                if v.iter().all(|x| x.is_finite())
                    && (-85.0..=85.0).contains(&v[0])
                    && (-180.0..=180.0).contains(&v[1])
                    && (20.0..=1500.0).contains(&v[2])
                    && (0.5..=32.0).contains(&v[3]) =>
            {
                error.set(String::new());
                Some(UiAction::ImportMap {
                    latitude: v[0],
                    longitude: v[1],
                    radius: v[2],
                    meters_per_stud: v[3],
                    endpoint: endpoint.get_untracked(),
                    local,
                    terrain: terrain.get_untracked(),
                    fit_height: fit_height.get_untracked(),
                    vertical_exaggeration: exaggeration.get_untracked(),
                })
            }
            _ => {
                let message =
                    "Use latitude ±85, longitude ±180, radius 20–1500 m, and 0.5–32 m per stud."
                        .to_string();
                error.set(message.clone());
                state.status.set(message);
                None
            }
        }
    };
    stack_panel().spacing(12.0).children((
        controls::label("GENERATE A REAL AREA"),
        controls::caption("Create the whole selected area: measured terrain, mapped buildings, roads and waterways, all as editable bricks."),
        button().name("TerrainSample").padding(12.0)
            .on_click(move |_| sample(UiAction::ImportTerrainSample))
            .child(stack_panel().spacing(4.0).children((
                text_block().text("San Francisco hills").font_size(14.0).font_weight(FontWeight::SemiBold),
                controls::caption("Real elevation + mapped streets · offline"),
            ))),
        grid().column_definitions("*,*").children((
            stack_panel().spacing(6.0).margin((0.0, 0.0, 4.0, 0.0)).children((
                controls::label("LATITUDE"),
                text_box().name("MapLatitude").text(latitude.two_way()).placeholder_text("52.516275"),
            )),
            stack_panel().grid_column(1).spacing(6.0).margin((4.0, 0.0, 0.0, 0.0)).children((
                controls::label("LONGITUDE"),
                text_box().name("MapLongitude").text(longitude.two_way()).placeholder_text("13.377704"),
            )),
        )),
        rerender(move || {
            let selected = radius.get();
            let buttons: Vec<Element> = [("160", "160 m radius"), ("500", "500 m radius"), ("1500", "1500 m radius")]
                .into_iter().enumerate().map(|(index, (value, label))| {
                    button().grid_column(index as i32).content(label).font_size(11.0)
                        .padding((4.0, 8.0)).margin((2.0, 0.0, 2.0, 0.0))
                        .background(if selected == value { "#533A29" } else { theme::INSET })
                        .border_brush(if selected == value { theme::ACCENT } else { theme::EDGE })
                        .tool_tip_tip(format!("{value} meter radius"))
                        .on_click(move |_| radius.set(value.into())).into_element()
                }).collect();
            grid().column_definitions("*,*,*").children(buttons).into_element()
        }),
        grid().column_definitions("*,*").children((
            stack_panel().spacing(6.0).margin((0.0, 0.0, 4.0, 0.0)).children((
                controls::label("RADIUS · METERS"),
                text_box().name("MapRadius").text(radius.two_way()).placeholder_text("160"),
            )),
            stack_panel().grid_column(1).spacing(6.0).margin((4.0, 0.0, 0.0, 0.0)).children((
                controls::label("METERS PER STUD"),
                text_box().name("MapScale").text(scale.two_way()).placeholder_text("4"),
            )),
        )),
        border().background("#203830").corner_radius(8.0).padding(12.0).child(
            text_block().name("MapAreaSummary").foreground("#BDE4CB").font_size(12.0)
                .font_weight(FontWeight::SemiBold).text_wrapping(TextWrapping::Wrap)
                .text(move || {
                    match (radius.get().parse::<f64>(), scale.get().parse::<f64>()) {
                        (Ok(r), Ok(s)) if r.is_finite() && s.is_finite() && r > 0.0 && s > 0.0 =>
                            format!("{:.0} × {:.0} m area\n{:.0} × {:.0} studs at this scale", r * 2.0, r * 2.0, r * 2.0 / s, r * 2.0 / s),
                        _ => "Choose an area and brick scale.".into(),
                    }
                })),
        stack_panel().spacing(2.0).children((
            check_box().name("UseRealElevation").content("Use real elevation")
                .is_checked(move || Some(terrain.get())).on_click(move |_| terrain.set(!terrain.get_untracked())),
            check_box().name("FitTerrainHeight").content("Fit terrain and building height")
                .is_enabled(terrain)
                .is_checked(move || Some(fit_height.get())).on_click(move |_| fit_height.set(!fit_height.get_untracked())),
        )),
        grid().column_definitions("*,*").is_enabled(terrain).children((
            button().content("Relief 1x").margin((0.0, 0.0, 4.0, 0.0))
                .background(move || if exaggeration.get() == 1.0 { "#533A29" } else { theme::INSET })
                .on_click(move |_| exaggeration.set(1.0)),
            button().grid_column(1).content("Relief 2x").margin((4.0, 0.0, 0.0, 0.0))
                .background(move || if exaggeration.get() == 2.0 { "#533A29" } else { theme::INSET })
                .on_click(move |_| exaggeration.set(2.0)),
        )),
        text_block().font_size(12.0).foreground(theme::MUTED).text_wrapping(TextWrapping::Wrap)
            .text(move || if terrain.get() {
                "1x keeps the source relief. Fit height scales terrain and buildings together to the available height; every adjustment is recorded. Turn it off to require exact vertical scale."
            } else {
                "Without elevation, mapped features use a flat base. Building heights use the selected scale; geometry that exceeds the available height is reported."
            }),
        stack_panel().spacing(12.0).children((
        text_block().text(error).foreground("#FFBEA3").font_size(12.0).text_wrapping(TextWrapping::Wrap)
            .is_visible(move || !error.get().is_empty()),
        button().name("GenerateMappedArea").class("primary").content("Generate mapped area")
            .on_click(move |_| { if let Some(request) = action(false) { generate(request); } }),
        button().class("quiet").content("Open local OSM / Overpass JSON")
            .on_click(move |_| { if let Some(request) = action(true) { local(request); } }),
        controls::caption("Local JSON uses the coordinates and radius above. Elevation downloads when enabled."),
        button().class("quiet").content(move || if advanced.get() { "Hide data source" } else { "Advanced data source" })
            .on_click(move |_| advanced.set(!advanced.get_untracked())),
        stack_panel().spacing(8.0).is_visible(advanced).children((
            text_box().name("OverpassEndpoint").text(endpoint.two_way()).placeholder_text("Overpass API endpoint"),
            controls::caption("Locations load only on request and are cached. Use your own endpoint for repeated imports."),
        )),
        controls::caption("Map data © OpenStreetMap contributors. Terrain comes from public elevation data; source credits and estimated building heights are kept in the world report."),
        button().content("Brandenburg Gate · included flat map")
            .on_click(move |_| landmark(UiAction::ImportLandmark)),
        )),
    )).into()
}
