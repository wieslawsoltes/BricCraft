//! Interactive display map composed from native image and shape elements.
use crate::{
    controls,
    icons::Icon,
    map_geometry as geo,
    map_gestures::{MapGestures, MapWheel},
    theme, Dispatch, UiAction, UiState,
};
use ferroui_base::{
    input::{Key, KeyModifiers},
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{FontWeight, Stretch},
};
use ferroui_controls::Border;
use ferroui_view::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn change(state: UiState, dispatch: &Dispatch, edit: impl FnOnce(&mut crate::MapView)) {
    let mut map = state.map_view.get_untracked();
    edit(&mut map);
    state.map_view.set_if_changed(map);
    dispatch(UiAction::MapViewportChanged);
}

fn select(state: UiState, point: [f64; 2], size: [f64; 2]) {
    let mut map = state.map_view.get_untracked();
    let location = geo::pick_position(&map, size, point);
    map.selected_latitude = location[0];
    map.selected_longitude = location[1];
    map.label = "Selected on map".into();
    state.map_view.set_if_changed(map);
}

fn tile_layer(state: UiState) -> Element {
    rerender(move || {
        let map = state.map_view.get();
        let size = geo::map_view_size(state.layout.get());
        let tiles = state.map_tiles.get();
        let children: Vec<Element> = tiles
            .iter()
            .filter(|tile| tile.zoom == map.zoom)
            .map(|tile| {
                let p = geo::tile_position(&map, size, tile.x, tile.y);
                image()
                    .source(Some(tile.image.clone()))
                    .stretch(Stretch::Fill)
                    .width(256.0)
                    .height(256.0)
                    .canvas_left(p[0])
                    .canvas_top(p[1])
                    .is_hit_test_visible(false)
                    .into_element()
            })
            .collect();
        canvas()
            .width(size[0])
            .height(size[1])
            .clip_to_bounds(true)
            .children(children)
            .is_hit_test_visible(false)
            .into_element()
    })
}

fn selection_layer(state: UiState) -> Element {
    rerender(move || {
        let map = state.map_view.get();
        let size = geo::map_view_size(state.layout.get());
        let point = geo::screen_position(&map, size, map.selected_latitude, map.selected_longitude);
        let diameter = (map.radius * 2.0 / geo::meters_per_pixel(map.selected_latitude, map.zoom))
            .clamp(2.0, 100_000.0);
        canvas()
            .is_hit_test_visible(false)
            .children((
                border()
                    .width(diameter)
                    .height(diameter)
                    .canvas_left(point[0] - diameter * 0.5)
                    .canvas_top(point[1] - diameter * 0.5)
                    .background("#26FFBD62")
                    .border_brush("#F09428")
                    .border_thickness(2.0),
                ellipse()
                    .width(24.0)
                    .height(24.0)
                    .fill("#E7392111")
                    .stroke("#FFFFFF")
                    .stroke_thickness(2.0)
                    .canvas_left(point[0] - 12.0)
                    .canvas_top(point[1] - 12.0),
                ellipse()
                    .width(10.0)
                    .height(10.0)
                    .fill(theme::ACCENT)
                    .canvas_left(point[0] - 5.0)
                    .canvas_top(point[1] - 5.0),
            ))
            .into_element()
    })
}

fn scale_bar(state: UiState) -> Element {
    rerender(move || {
        let map = state.map_view.get();
        let mpp = geo::meters_per_pixel(map.latitude, map.zoom);
        let desired = mpp * 90.0;
        let power = 10_f64.powf(desired.log10().floor());
        let distance = [5.0, 2.0, 1.0]
            .into_iter()
            .map(|n| n * power)
            .find(|value| *value <= desired)
            .unwrap_or(power);
        let label = if distance >= 1000.0 {
            format!("{:.0} km", distance / 1000.0)
        } else {
            format!("{distance:.0} m")
        };
        border()
            .background("#E8172735")
            .padding((8.0, 5.0))
            .corner_radius(5.0)
            .horizontal_alignment(H::Left)
            .vertical_alignment(V::Bottom)
            .margin((8.0, 0.0, 0.0, 40.0))
            .is_hit_test_visible(false)
            .child(
                stack_panel().spacing(3.0).children((
                    text_block()
                        .text(label)
                        .font_size(10.0)
                        .font_weight(FontWeight::SemiBold),
                    border()
                        .width((distance / mpp).clamp(10.0, 90.0))
                        .height(5.0)
                        .border_brush("#FFFFFF")
                        .border_thickness((1.0, 0.0, 1.0, 2.0)),
                )),
            )
            .into_element()
    })
}

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    let viewport: NodeRef<Border> = NodeRef::new();
    let gestures = Rc::new(RefCell::new(MapGestures::default()));
    let pressed = gestures.clone();
    let moved = gestures.clone();
    let released = gestures.clone();
    let lost = gestures;
    let move_dispatch = dispatch.clone();
    let wheel_dispatch = dispatch.clone();
    let wheel_input = RefCell::new(MapWheel::default());
    let keyboard_dispatch = dispatch.clone();
    let zoom_in = dispatch.clone();
    let zoom_out = dispatch.clone();
    let center = dispatch.clone();
    let credit = dispatch;
    grid()
        .name("PlaceMap")
        .width(move || geo::map_view_size(state.layout.get())[0])
        .height(move || geo::map_view_size(state.layout.get())[1])
        .clip_to_bounds(true)
        .children((
            border()
                .name("PlaceMapViewport")
                .node_ref(viewport)
                .background("#263D47")
                .clip_to_bounds(true)
                .focusable(true)
                .automation_properties_name(Some(
                    "Map. Drag to pan, scroll or pinch to zoom, tap to choose a place.".into(),
                ))
                .automation_properties_help_text(Some(
                    "Arrow keys pan. Page Up and Page Down zoom. Enter selects the map center."
                        .into(),
                ))
                .child(grid().children((tile_layer(state), selection_layer(state))))
                .on_pointer_pressed(move |event| {
                    // The phone layout places this viewport inside a ScrollViewer.
                    // Its gesture class handlers observe handled events as well.
                    event.prevent_gesture_recognition();
                    if !state.places_open.get_untracked() {
                        return;
                    }
                    let Some(view) = viewport.get_untracked() else {
                        return;
                    };
                    let point = event.get_position(Some(&view));
                    let _ = view.focus();
                    pressed
                        .borrow_mut()
                        .press(event.pointer().id(), [point.x, point.y]);
                    event.pointer().capture(Some(&view.clone().upcast()));
                    event.set_handled(true);
                })
                .on_pointer_moved(move |event| {
                    event.prevent_gesture_recognition();
                    if !state.places_open.get_untracked() {
                        moved.borrow_mut().cancel(event.pointer().id());
                        event.pointer().capture(None);
                        event.set_handled(true);
                        return;
                    }
                    let Some(view) = viewport.get_untracked() else {
                        return;
                    };
                    let point = event.get_position(Some(&view));
                    let movement = moved
                        .borrow_mut()
                        .moved(event.pointer().id(), [point.x, point.y]);
                    if let Some(movement) = movement {
                        let size = geo::map_view_size(state.layout.get_untracked());
                        change(state, &move_dispatch, |map| {
                            geo::pan(map, movement.pan);
                            if movement.zoom != 0 {
                                geo::zoom_at(map, size, movement.anchor, movement.zoom);
                            }
                        });
                    }
                    event.set_handled(true);
                })
                .on_pointer_released(move |event| {
                    event.prevent_gesture_recognition();
                    let tap = released.borrow_mut().release(event.pointer().id());
                    if tap && state.places_open.get_untracked() {
                        if let Some(view) = viewport.get_untracked() {
                            let point = event.get_position(Some(&view));
                            select(
                                state,
                                [point.x, point.y],
                                geo::map_view_size(state.layout.get_untracked()),
                            );
                        }
                    }
                    event.pointer().capture(None);
                    event.set_handled(true);
                })
                .on_pointer_capture_lost(move |event| {
                    lost.borrow_mut().cancel(event.pointer().id())
                })
                .on_pointer_wheel_changed(move |event| {
                    if !state.places_open.get_untracked() {
                        return;
                    }
                    let Some(view) = viewport.get_untracked() else {
                        return;
                    };
                    let point = event.get_position(Some(&view));
                    let delta = event.delta();
                    let size = geo::map_view_size(state.layout.get_untracked());
                    let pan = event.key_modifiers().contains(KeyModifiers::SHIFT)
                        || delta.x.abs() > delta.y.abs() * 0.25;
                    let zoom = if pan {
                        0
                    } else {
                        wheel_input.borrow_mut().consume(delta.y)
                    };
                    if pan || zoom != 0 {
                        change(state, &wheel_dispatch, |map| {
                            if pan {
                                geo::pan(map, [-delta.x * 36.0, delta.y * 36.0]);
                            } else {
                                geo::zoom_at(map, size, [point.x, point.y], zoom);
                            }
                        });
                    }
                    event.set_handled(true);
                })
                .on_key_down(move |event| {
                    if !state.places_open.get_untracked() {
                        return;
                    }
                    let size = geo::map_view_size(state.layout.get_untracked());
                    let key = event.key;
                    if key == Key::Return || key == Key::Space {
                        select(state, [size[0] * 0.5, size[1] * 0.5], size);
                    } else {
                        let action = match key {
                            Key::Left => Some(([96.0, 0.0], 0)),
                            Key::Right => Some(([-96.0, 0.0], 0)),
                            Key::Up => Some(([0.0, 96.0], 0)),
                            Key::Down => Some(([0.0, -96.0], 0)),
                            Key::PageUp => Some(([0.0, 0.0], 1)),
                            Key::PageDown => Some(([0.0, 0.0], -1)),
                            _ => None,
                        };
                        let Some((pan, zoom)) = action else {
                            return;
                        };
                        change(state, &keyboard_dispatch, |map| {
                            geo::pan(map, pan);
                            geo::zoom_at(map, size, [size[0] * 0.5, size[1] * 0.5], zoom);
                        });
                    }
                    event.set_handled(true);
                }),
            stack_panel()
                .horizontal_alignment(H::Right)
                .vertical_alignment(V::Top)
                .margin(8.0)
                .spacing(5.0)
                .children((
                    button()
                        .name("MapZoomIn")
                        .width(44.0)
                        .height(44.0)
                        .padding(0.0)
                        .background(theme::HUD)
                        .content("+")
                        .font_size(24.0)
                        .automation_properties_name(Some("Zoom in on map".into()))
                        .is_enabled(move || state.map_view.get().zoom < geo::MAX_MAP_ZOOM)
                        .on_click(move |_| {
                            let size = geo::map_view_size(state.layout.get_untracked());
                            change(state, &zoom_in, |map| {
                                geo::zoom_at(map, size, [size[0] * 0.5, size[1] * 0.5], 1)
                            });
                        }),
                    button()
                        .name("MapZoomOut")
                        .width(44.0)
                        .height(44.0)
                        .padding(0.0)
                        .background(theme::HUD)
                        .content("−")
                        .font_size(24.0)
                        .automation_properties_name(Some("Zoom out on map".into()))
                        .is_enabled(move || state.map_view.get().zoom > geo::MIN_MAP_ZOOM)
                        .on_click(move |_| {
                            let size = geo::map_view_size(state.layout.get_untracked());
                            change(state, &zoom_out, |map| {
                                geo::zoom_at(map, size, [size[0] * 0.5, size[1] * 0.5], -1)
                            });
                        }),
                    controls::tool_button(
                        "MapRecenter",
                        "Return to selected place",
                        Icon::Goals,
                        move || {
                            change(state, &center, |map| {
                                map.latitude = map.selected_latitude;
                                map.longitude = map.selected_longitude;
                            });
                        },
                    )
                    .background(theme::HUD),
                )),
            scale_bar(state),
            button()
                .name("MapAttribution")
                .class("quiet")
                .background("#EB172735")
                .vertical_alignment(V::Bottom)
                .horizontal_alignment(H::Right)
                .min_height(32.0)
                .padding((8.0, 5.0))
                .corner_radius(0.0)
                .content("© OpenStreetMap contributors")
                .font_size(10.0)
                .on_click(move |_| {
                    credit(UiAction::OpenSource(
                        "https://www.openstreetmap.org/copyright".into(),
                    ))
                }),
        ))
        .into()
}
