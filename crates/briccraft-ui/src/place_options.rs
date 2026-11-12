//! Location choices and the two map-to-world workflows.
use crate::{controls, map_location::MAP_PRESETS, theme, Dispatch, UiAction, UiState};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

fn choices(state: UiState) -> Element {
    stack_panel()
        .spacing(9.0)
        .children((
            controls::label("BUILD AREA · RADIUS"),
            rerender(move || {
                let selected = state.map_view.get().radius;
                let choices: Vec<Element> =
                    [(160.0, "160 m"), (500.0, "500 m"), (1500.0, "1.5 km")]
                        .into_iter()
                        .enumerate()
                        .map(|(index, (radius, label))| {
                            button()
                                .content(label)
                                .grid_column(index as i32)
                                .padding((3.0, 8.0))
                                .margin((2.0, 0.0))
                                .font_size(11.0)
                                .background(if radius == selected {
                                    theme::SELECTED
                                } else {
                                    theme::INSET
                                })
                                .border_brush(if radius == selected {
                                    theme::ACCENT
                                } else {
                                    theme::EDGE
                                })
                                .on_click(move |_| {
                                    let mut map = state.map_view.get_untracked();
                                    map.radius = radius;
                                    state.map_view.set(map);
                                })
                                .into_element()
                        })
                        .collect();
                grid()
                    .column_definitions("*,*,*")
                    .children(choices)
                    .into_element()
            }),
            controls::label("WORLD SCALE · METERS PER STUD"),
            rerender(move || {
                let selected = state.map_view.get().meters_per_stud;
                let choices: Vec<Element> = [(2.0, "2 m"), (4.0, "4 m"), (8.0, "8 m")]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (scale, label))| {
                        button()
                            .content(label)
                            .grid_column(index as i32)
                            .padding((3.0, 8.0))
                            .margin((2.0, 0.0))
                            .font_size(11.0)
                            .background(if scale == selected {
                                theme::SELECTED
                            } else {
                                theme::INSET
                            })
                            .border_brush(if scale == selected {
                                theme::ACCENT
                            } else {
                                theme::EDGE
                            })
                            .on_click(move |_| {
                                let mut map = state.map_view.get_untracked();
                                map.meters_per_stud = scale;
                                state.map_view.set(map);
                            })
                            .into_element()
                    })
                    .collect();
                grid()
                    .column_definitions("*,*,*")
                    .children(choices)
                    .into_element()
            }),
            check_box()
                .name("MapElevation")
                .content("Use real terrain elevation")
                .is_checked(move || Some(state.map_view.get().terrain))
                .on_click(move |_| {
                    let mut map = state.map_view.get_untracked();
                    map.terrain = !map.terrain;
                    state.map_view.set(map);
                }),
        ))
        .into()
}

fn coordinates(state: UiState, dispatch: Dispatch) -> Element {
    let latitude = signal(String::new());
    let longitude = signal(String::new());
    let expanded = signal(false);
    let error = signal(String::new());
    effect(move || {
        let map = state.map_view.get();
        latitude.set_if_changed(format!("{:.6}", map.selected_latitude));
        longitude.set_if_changed(format!("{:.6}", map.selected_longitude));
    });
    let go = dispatch;
    stack_panel()
        .spacing(8.0)
        .children((
            button()
                .name("MapCoordinates")
                .class("quiet")
                .content(move || {
                    if expanded.get() {
                        "Hide coordinates"
                    } else {
                        "Go to coordinates…"
                    }
                })
                .on_click(move |_| expanded.set(!expanded.get_untracked())),
            stack_panel().spacing(8.0).is_visible(expanded).children((
                grid().column_definitions("*,*").children((
                    stack_panel()
                        .spacing(5.0)
                        .margin((0.0, 0.0, 4.0, 0.0))
                        .children((
                            controls::label("LATITUDE"),
                            text_box()
                                .name("PlaceLatitude")
                                .text(latitude.two_way())
                                .automation_properties_name(Some("Map latitude".into())),
                        )),
                    stack_panel()
                        .grid_column(1)
                        .spacing(5.0)
                        .margin((4.0, 0.0, 0.0, 0.0))
                        .children((
                            controls::label("LONGITUDE"),
                            text_box()
                                .name("PlaceLongitude")
                                .text(longitude.two_way())
                                .automation_properties_name(Some("Map longitude".into())),
                        )),
                )),
                text_block()
                    .text(error)
                    .foreground("#FFBEA3")
                    .font_size(11.0)
                    .is_visible(move || !error.get().is_empty())
                    .text_wrapping(TextWrapping::Wrap),
                button()
                    .name("MapGo")
                    .content("Show this location")
                    .on_click(move |_| {
                        let parsed = (
                            latitude.get_untracked().trim().parse::<f64>(),
                            longitude.get_untracked().trim().parse::<f64>(),
                        );
                        if let (Ok(lat), Ok(lon)) = parsed {
                            if lat.is_finite()
                                && lon.is_finite()
                                && (-85.0..=85.0).contains(&lat)
                                && (-180.0..=180.0).contains(&lon)
                            {
                                let mut map = state.map_view.get_untracked();
                                map.latitude = lat;
                                map.longitude = lon;
                                map.selected_latitude = lat;
                                map.selected_longitude = lon;
                                map.label = "Your location".into();
                                map.zoom = map.zoom.max(14);
                                error.set(String::new());
                                state.map_view.set(map);
                                go(UiAction::MapViewportChanged);
                                return;
                            }
                        }
                        error.set(
                            "Enter latitude from −85 to 85 and longitude from −180 to 180.".into(),
                        );
                    }),
            )),
        ))
        .into()
}

pub fn presets(state: UiState, dispatch: Dispatch) -> Element {
    let presets: Vec<Element> = MAP_PRESETS
        .into_iter()
        .enumerate()
        .map(|(index, (label, latitude, longitude))| {
            let changed = dispatch.clone();
            button()
                .name(format!("MapPreset{index}"))
                .content(label)
                .grid_row((index / 2) as i32)
                .grid_column((index % 2) as i32)
                .margin(3.0)
                .padding((5.0, 8.0))
                .font_size(11.0)
                .on_click(move |_| {
                    let mut map = state.map_view.get_untracked();
                    map.latitude = latitude;
                    map.longitude = longitude;
                    map.selected_latitude = latitude;
                    map.selected_longitude = longitude;
                    map.zoom = 16;
                    map.label = label.into();
                    state.map_view.set(map);
                    changed(UiAction::MapViewportChanged);
                })
                .into_element()
        })
        .collect();
    grid()
        .column_definitions("*,*")
        .row_definitions("Auto,Auto")
        .children(presets)
        .into()
}

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    let build = dispatch.clone();
    let roam = dispatch.clone();
    let retry = dispatch.clone();
    let stop = dispatch.clone();
    stack_panel().spacing(13.0).children((
        stack_panel().spacing(5.0).children((
            controls::label("YOUR SELECTED PLACE"),
            text_block().text(move || state.map_view.get().label).font_size(18.0)
                .font_weight(FontWeight::SemiBold).text_wrapping(TextWrapping::Wrap),
            text_block().text(move || {
                let map=state.map_view.get();
                format!("{:.5}, {:.5}",map.selected_latitude,map.selected_longitude)
            }).font_size(12.0).foreground(theme::MUTED),
            text_block().name("SelectedAreaStatus")
                .is_visible(move || !state.import_busy.get() && !state.import_status.get().is_empty())
                .text(state.import_status).font_size(11.0).foreground(theme::MUTED)
                .text_wrapping(TextWrapping::Wrap),
        )),
        button().name("BuildMapSelection").class("primary").min_height(48.0)
            .content("Build this area").is_enabled(move || !state.import_busy.get() && !state.world_busy.get())
            .on_click(move |_| {
                let map=state.map_view.get_untracked();
                state.import_status.set(String::new());
                build(UiAction::ImportMap {
                    latitude:map.selected_latitude,longitude:map.selected_longitude,radius:map.radius,
                    meters_per_stud:map.meters_per_stud,endpoint:"https://overpass-api.de/api/interpreter".into(),
                    local:false,terrain:map.terrain,fit_height:true,vertical_exaggeration:1.0,
                });
            }),
        button().name("StartMapRoam").min_height(48.0).background("#214239")
            .border_brush("#6BB49A").foreground("#C6F6E1").content("Free roam from here")
            .is_enabled(move || !state.import_busy.get() && !state.world_busy.get())
            .on_click(move |_| {
                let map=state.map_view.get_untracked();
                roam(UiAction::StartMapRoam {
                    latitude:map.selected_latitude,longitude:map.selected_longitude,
                    meters_per_stud:map.meters_per_stud,terrain:map.terrain,
                });
            }),
        controls::caption("Build a complete selected area, or walk freely while nearby mapped streets and terrain generate around you."),
        border().background(theme::EDGE).height(1.0),
        choices(state),
        text_block().font_size(11.0).foreground(theme::MUTED).text_wrapping(TextWrapping::Wrap)
            .text(move || {
                let map=state.map_view.get();
                format!("Area: {:.0} × {:.0} m · {:.0} × {:.0} studs. Free roam continues beyond this selection.",map.radius*2.0,map.radius*2.0,map.radius*2.0/map.meters_per_stud,map.radius*2.0/map.meters_per_stud)
            }),
        border().is_visible(move || !state.roam_status.get().is_empty()).background("#17392F").corner_radius(10.0).padding(12.0)
            .child(stack_panel().spacing(8.0).children((
                controls::label("FREE ROAM"),
                text_block().text(state.roam_status).foreground(theme::POSITIVE).font_size(12.0).text_wrapping(TextWrapping::Wrap),
                grid().column_definitions("*,*").children((
                    button().name("RetryMapRoam").content("Resume / retry").padding((4.0,8.0)).margin((0.0,0.0,4.0,0.0))
                        .on_click(move |_| retry(UiAction::RetryMapRoam)),
                    button().name("StopMapRoam").grid_column(1).content("Pause streaming").padding((4.0,8.0)).is_enabled(state.roam_active)
                        .on_click(move |_| stop(UiAction::StopMapRoam)),
                )),
            ))),
        controls::label("PLACES TO EXPLORE"),
        presets(state,dispatch.clone()),
        coordinates(state,dispatch),
        button().name("OpenAdvancedStudio").class("quiet").content("3D models & advanced imports")
            .on_click(move |_| {state.close_panels();state.imports_open.set(true);}),
    )).into()
}
