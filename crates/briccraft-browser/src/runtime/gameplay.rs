//! A small presentation boundary keeps the native UI independent of game rules.
use briccraft_game::Session;
use briccraft_ui::{BlueprintQuote, GameplayView, ObjectiveCard, WorkshopView};

const BLUEPRINTS: [&str; 8] = [
    "rover", "plane", "train", "boat", "house", "tower", "bridge", "windmill",
];

pub(super) fn project(game: &Session) -> GameplayView {
    let selected = game.selected_part_quote();
    let workshop = game.workshop_status();
    GameplayView {
        expedition: game.gameplay.is_expedition(),
        started: game.gameplay.started(),
        studs: game.gameplay.studs(),
        selected_cost: selected.studs,
        selected_affordable: selected.affordable,
        objectives: game
            .gameplay
            .objectives()
            .into_iter()
            .map(|goal| ObjectiveCard {
                id: goal.id.into(),
                title: goal.title.into(),
                detail: goal.detail.into(),
                progress: goal.progress,
                target: goal.target,
                reward: goal.reward,
                completed: goal.completed,
            })
            .collect(),
        blueprints: BLUEPRINTS
            .into_iter()
            .filter_map(|id| {
                game.blueprint_quote(id).map(|quote| BlueprintQuote {
                    id: id.into(),
                    studs: quote.studs,
                    affordable: quote.affordable,
                })
            })
            .collect(),
        guidance: game.gameplay.guidance().into(),
        workshop: WorkshopView {
            title: workshop.title,
            detail: workshop.detail,
            ready: workshop.ready,
        },
    }
}

/// Diagnostics query the authoritative game state, including immediately after
/// a command, without waiting for the less frequent native HUD projection.
pub(crate) fn diagnostics(game: &Session) -> serde_json::Value {
    let quote = game.selected_part_quote();
    let workshop = game.workshop_status();
    let objectives: Vec<_> = game
        .gameplay
        .objectives()
        .into_iter()
        .map(|goal| {
            serde_json::json!({
                "id":goal.id,"title":goal.title,"detail":goal.detail,"progress":goal.progress,
                "target":goal.target,"reward":goal.reward,"completed":goal.completed,
            })
        })
        .collect();
    let blueprints: Vec<_> = BLUEPRINTS
        .into_iter()
        .filter_map(|name| {
            game.blueprint_quote(name).map(|cost| serde_json::json!({
        "name":name,"studs":cost.studs,"affordable":cost.affordable,"charged":cost.charged,
    }))
        })
        .collect();
    serde_json::json!({
        "mode":if game.gameplay.is_expedition() { "expedition" } else { "creative" },
        "studs":game.gameplay.studs(),"started":game.gameplay.started(),
        "selected":{"studs":quote.studs,"affordable":quote.affordable,"charged":quote.charged},
        "objectives":objectives,"guidance":game.gameplay.guidance(),
        "blueprints":blueprints,
        "workshop":{"title":workshop.title,"detail":workshop.detail,"ready":workshop.ready},
    })
}
