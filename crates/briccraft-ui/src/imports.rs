//! User-initiated geographic and mesh imports remain native FerroUI controls.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let latitude = signal("52.516275".to_string());
    let longitude = signal("13.377704".to_string());
    let radius = signal("160".to_string());
    let scale = signal("4".to_string());
    let endpoint = signal("https://overpass-api.de/api/interpreter".to_string());
    let model_size = signal(48.0);
    let map_scale = signal(false);
    let map = dispatch.clone();
    let local = dispatch.clone();
    let landmark = dispatch.clone();
    let model = dispatch.clone();
    let append = dispatch.clone();
    let report = dispatch.clone();
    let sample = dispatch;
    let map_action = move |local: bool| -> Option<UiAction> {
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
                Some(UiAction::ImportMap {
                    latitude: v[0],
                    longitude: v[1],
                    radius: v[2],
                    meters_per_stud: v[3],
                    endpoint: endpoint.get_untracked(),
                    local,
                })
            }
            _ => {
                state.status.set(
                    "Use latitude ±85, longitude ±180, radius 20–1500 m, and 0.5–32 m per stud."
                        .into(),
                );
                None
            }
        }
    };
    let content = scroll_viewer().horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(12.0).children((
            stack_panel().spacing(12.0).children((
            controls::caption("Turn a real location or a 3D model into editable bricks. Open a new world, or add a model to your current streamed build."),
            button().class("primary").content("Brandenburg Gate · included map").on_click(move |_| landmark(UiAction::ImportLandmark)),
            controls::label("OPENSTREETMAP · REAL PLACES"),
            grid().column_definitions("*,*").children((
                text_box().name("MapLatitude").text(latitude.two_way()).placeholder_text("Latitude").margin((0.0,0.0,4.0,0.0)),
                text_box().name("MapLongitude").grid_column(1).text(longitude.two_way()).placeholder_text("Longitude").margin((4.0,0.0,0.0,0.0)),
            )),
            controls::caption("Area radius in meters · meters represented by one stud"),
            grid().column_definitions("*,*").children((
                text_box().name("MapRadius").text(radius.two_way()).placeholder_text("Radius, meters").margin((0.0,0.0,4.0,0.0)),
                text_box().name("MapScale").grid_column(1).text(scale.two_way()).placeholder_text("Meters per stud").margin((4.0,0.0,0.0,0.0)),
            )),
            text_box().name("OverpassEndpoint").text(endpoint.two_way()).placeholder_text("Your Overpass API endpoint"),
            button().content("Import this location").on_click(move |_| { if let Some(action)=map_action(false) { map(action); } }),
            button().class("quiet").content("Open a local OSM / Overpass JSON extract").on_click(move |_| { if let Some(action)=map_action(true) { local(action); } }),
            controls::caption("Footprints, roads, waterways and recorded heights become bricks. Missing heights are marked as estimates. Larger meters per stud preserve proportions within the 72-plate height budget."),
            controls::caption("Locations load on request and are cached. Use your own endpoint for repeated imports. Map data © OpenStreetMap contributors · ODbL."),
            )),
            border().height(1.0).background(theme::EDGE).margin((0.0,5.0)),
            stack_panel().spacing(12.0).children((
            controls::label("3D MODELS · REAL GEOMETRY"),
            controls::caption("Import a GLB or OBJ model. Scene transforms and material colors are converted to a stud-and-plate surface, then packed into real catalog pieces."),
            rerender(move || {
                let selected=model_size.get();
                let buttons:Vec<Element>=[(24.0,"24 studs"),(48.0,"48 studs"),(64.0,"64 studs")].into_iter().map(|(value,label)|
                    button().content(label).font_size(11.0).padding((9.0,7.0))
                        .background(if value==selected { "#533A29" } else { theme::INSET })
                        .on_click(move |_| model_size.set(value)).into_element()
                ).collect();
                stack_panel().orientation(Orientation::Horizontal).spacing(5.0).children(buttons).into_element()
            }),
            button().content("Open 3D model file").on_click(move |_| model(UiAction::ImportModel{size:model_size.get_untracked(),sample:false})),
            stack_panel().spacing(8.0).is_visible(state.streamed).children((
                check_box().content("Use this map's meters per stud").is_checked(move || Some(map_scale.get()))
                    .on_click(move |_| map_scale.set(!map_scale.get_untracked())),
                button().content("Add a model to this world").on_click(move |_| append(UiAction::AppendModel{size:model_size.get_untracked(),map_scale:map_scale.get_untracked()})),
                controls::caption("Place at the current construction target. Meter scale aligns a glTF model's physical dimensions with the map. Choose a clear area; overlapping imports are rejected."),
            )),
            button().class("quiet").content("Try Khronos ToyCar · CC0").on_click(move |_| sample(UiAction::ImportModel{size:model_size.get_untracked(),sample:true})),
            controls::caption("Models are fitted uniformly to the selected size and available height. Fine texture details become palette colors; the import report lists unsupported features."),
            text_block().text(state.import_status).foreground(theme::MUTED).font_size(11.0).text_wrapping(TextWrapping::Wrap),
            button().class("quiet").content("Export this world's source report").on_click(move |_| report(UiAction::ExportReport)),
            )),
        ))).into();
    border()
        .is_visible(state.imports_open)
        .child(controls::panel(
            "Real-world studio",
            "FROM THE WORLD. INTO BRICKS.",
            content,
            move || state.imports_open.set(false),
            state,
        ))
        .into()
}

pub fn progress(state: UiState, dispatch: Dispatch) -> Element {
    border()
        .name("ImportProgress")
        .is_visible(move || state.import_busy.get() || state.world_busy.get())
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .width(move || if state.compact.get() { 290.0 } else { 420.0 })
        .padding(24.0)
        .corner_radius(16.0)
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .effect(theme::shadow())
        .child(
            stack_panel().spacing(14.0).children((
                controls::label("EVERY PIECE HAS A PLACE"),
                text_block()
                    .text(move || {
                        if state.world_busy.get() {
                            "Opening your world"
                        } else {
                            "Building your world"
                        }
                    })
                    .font_size(24.0)
                    .font_weight(FontWeight::SemiBold),
                text_block()
                    .text(move || {
                        if state.world_busy.get() {
                            state.status.get()
                        } else {
                            state.import_status.get()
                        }
                    })
                    .font_size(12.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                progress_bar()
                    .is_indeterminate(state.world_busy)
                    .minimum(0.0)
                    .maximum(1.0)
                    .value(state.import_progress)
                    .height(6.0),
                button()
                    .is_visible(state.import_busy)
                    .class("quiet")
                    .content("Cancel import")
                    .on_click(move |_| dispatch(UiAction::CancelImport)),
            )),
        )
        .into()
}
