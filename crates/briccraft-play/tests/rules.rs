use briccraft_play::*;

fn expedition() -> Gameplay {
    let mut g = Gameplay::default();
    g.set_expedition(true);
    g
}

#[test]
fn catalog_quotes_use_actual_material_and_functional_parts() {
    assert_eq!(part_cost(20), Some(1));
    assert_eq!(part_cost(3), Some(8));
    assert_eq!(part_cost(140), Some(100));
    assert_eq!(part_cost(160), Some(73));
    assert!(briccraft_parts::CATALOG
        .iter()
        .all(|p| part_cost(p.id).unwrap() > 0));
    assert_eq!(parts_cost([20, 3, 140, 160]).unwrap(), 182);
    assert!(parts_cost([u16::MAX]).is_err());
}

#[test]
fn approvals_do_not_spend_and_cannot_be_consumed_after_another_change() {
    let mut g = expedition();
    assert!(matches!(
        g.approve(ChangeValue {
            added: STARTER_STUDS + 1,
            removed: 0
        }),
        Err(PlayError::InsufficientStuds { .. })
    ));
    let unused = g
        .approve(ChangeValue {
            added: 10,
            removed: 0,
        })
        .unwrap();
    drop(unused);
    assert_eq!(g.studs(), STARTER_STUDS);
    let a = g
        .approve(ChangeValue {
            added: 10,
            removed: 0,
        })
        .unwrap();
    let b = g
        .approve(ChangeValue {
            added: 10,
            removed: 0,
        })
        .unwrap();
    g.commit(a).unwrap();
    assert_eq!(g.commit(b), Err(PlayError::StalePermit));
    assert_eq!(g.studs(), STARTER_STUDS - 10);
}

#[test]
fn recycling_and_inverse_changes_conserve_resources_and_require_repayment() {
    let mut g = expedition();
    let change = ChangeValue {
        added: 100,
        removed: 25,
    };
    let p = g.approve(change).unwrap();
    g.commit(p).unwrap();
    let p = g.approve(change.inverse()).unwrap();
    g.commit(p).unwrap();
    assert_eq!(g.studs(), STARTER_STUDS);
    let p = g
        .approve(ChangeValue {
            added: 0,
            removed: 30,
        })
        .unwrap();
    g.commit(p).unwrap();
    let p = g
        .approve(ChangeValue {
            added: g.studs(),
            removed: 0,
        })
        .unwrap();
    g.commit(p).unwrap();
    assert!(g
        .approve(ChangeValue {
            added: 30,
            removed: 0
        })
        .is_err());
    assert_eq!(g.studs(), 0);
}

#[test]
fn objectives_reward_once_across_repeated_actions_mode_changes_and_saved_state() {
    let mut g = expedition();
    let reward = g.record(Activity::Placed(24));
    assert_eq!(reward.studs, 180);
    assert_eq!(g.record(Activity::Placed(24)).studs, 0);
    let saved = serde_json::to_string(&g).unwrap();
    let mut restored: Gameplay = serde_json::from_str(&saved).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.record(Activity::Placed(24)).studs, 0);
    let balance = restored.studs();
    restored.set_expedition(false);
    restored.record(Activity::Recycled(100));
    assert_eq!(
        restored
            .objectives()
            .iter()
            .find(|o| o.id == "salvage")
            .unwrap()
            .progress,
        0
    );
    restored.set_expedition(true);
    assert_eq!(restored.studs(), balance);
    assert_eq!(restored.record(Activity::Recycled(8)).studs, 140);
}

#[test]
fn actual_step_distances_progress_but_teleports_and_invalid_values_do_not() {
    let mut g = expedition();
    for distance in [f64::NAN, f64::INFINITY, -1.0, 1000.0] {
        g.record(Activity::Travel {
            kind: TravelKind::Walk,
            studs: distance,
        });
    }
    assert_eq!(
        g.objectives()
            .iter()
            .find(|o| o.id == "walk")
            .unwrap()
            .progress,
        0
    );
    for _ in 0..30 {
        g.record(Activity::Travel {
            kind: TravelKind::Walk,
            studs: 4.0,
        });
    }
    let walk = g.objectives().into_iter().find(|o| o.id == "walk").unwrap();
    assert!(walk.completed);
    assert_eq!(walk.progress, 120);
    assert_eq!(g.studs(), STARTER_STUDS + 120);
}

#[test]
fn creative_is_unlimited_and_malformed_saved_limits_are_rejected() {
    let mut creative = Gameplay::default();
    let permit = creative
        .approve(ChangeValue {
            added: u64::MAX,
            removed: 0,
        })
        .unwrap();
    creative.commit(permit).unwrap();
    assert_eq!(creative.studs(), STARTER_STUDS);
    assert_eq!(creative.record(Activity::Activated).studs, 0);
    for (field, value) in [
        ("studs", serde_json::json!(u64::MAX)),
        ("claimed", serde_json::json!(256)),
        ("version", serde_json::json!(99)),
    ] {
        let mut source = serde_json::to_value(expedition()).unwrap();
        source[field] = value;
        let candidate: Gameplay = serde_json::from_value(source).unwrap();
        assert!(candidate.validate().is_err());
    }
}
