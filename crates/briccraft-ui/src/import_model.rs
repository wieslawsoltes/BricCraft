//! Local geometry and sample-model import controls.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    let size = signal(48.0);
    let map_scale = signal(false);
    let open = dispatch.clone();
    let append = dispatch.clone();
    let sample = dispatch;
    stack_panel().spacing(12.0).children((
        controls::label("3D MODELS · REAL GEOMETRY"),
        controls::caption("Import a GLB or OBJ model. Scene transforms and material colors become a stud-and-plate surface, packed into catalog pieces."),
        rerender(move || {
            let selected = size.get();
            let buttons: Vec<Element> = [(24.0, "24 studs"), (48.0, "48 studs"), (64.0, "64 studs")]
                .into_iter().map(|(value, title)| button().content(title).font_size(11.0).padding((9.0, 7.0))
                    .background(if value == selected { "#533A29" } else { theme::INSET })
                    .on_click(move |_| size.set(value)).into_element()).collect();
            stack_panel().orientation(Orientation::Horizontal).spacing(5.0).children(buttons).into_element()
        }),
        button().content("Open 3D model file")
            .on_click(move |_| open(UiAction::ImportModel { size: size.get_untracked(), sample: false })),
        stack_panel().spacing(8.0).is_visible(state.streamed).children((
            check_box().content("Use this map's meters per stud").is_checked(move || Some(map_scale.get()))
                .on_click(move |_| map_scale.set(!map_scale.get_untracked())),
            button().content("Add a model to this world")
                .on_click(move |_| append(UiAction::AppendModel { size: size.get_untracked(), map_scale: map_scale.get_untracked() })),
            controls::caption("Place at the construction target. Meter scale aligns a glTF model's dimensions with the map. Choose a clear area; overlapping imports are rejected."),
        )),
        button().class("quiet").content("Try Khronos ToyCar · CC0")
            .on_click(move |_| sample(UiAction::ImportModel { size: size.get_untracked(), sample: true })),
        controls::caption("Models fit uniformly to the selected size and height. Texture details become palette colors; the source report lists unsupported features."),
    )).into()
}
