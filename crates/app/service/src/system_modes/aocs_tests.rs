//! モードの結果を、同じ姿勢則を選ぶ通常解析と比較する。
use super::*;
use crate::dto::{AnalysisRunStatus, SetText};
use fune_model::role::{Role, STATE_OFF};
use sat_analysis::aocs::{RwUnloading, SensorFov, UnloadTime};
use sat_analysis::catalog::run_typed;
use sat_analysis::spec::Verdict;

fn demo() -> Workspace {
    Workspace::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"),
    )
    .unwrap()
}

#[test]
fn every_mode_matches_the_same_law_as_case_attitude() {
    let s = demo().snapshot().unwrap();
    let default = s.design.cases["hot_imaging"].clone();
    let cards = system_modes_of(&s, sat_analysis::spec::System::Aocs).unwrap();
    let mut momenta = Vec::new();
    for card in cards.modes {
        let mut case = default.clone();
        case.mode = Some(card.id.clone().into());
        let mut ctx = fune_projection::Context::bare(&s.design, &s.resolved);
        ctx.case = Some(&case);
        let rw =
            run_typed::<RwUnloading>(&fune_projection::aocs::rw_unloading(&ctx).unwrap()).unwrap();
        let fov =
            run_typed::<SensorFov>(&fune_projection::aocs::sensor_fov(&ctx).unwrap()).unwrap();
        let unload =
            run_typed::<UnloadTime>(&fune_projection::aocs::unload_time(&ctx).unwrap()).unwrap();
        let view = card.aocs.unwrap();
        assert_eq!(view.rw.status, AnalysisRunStatus::Ok);
        assert_eq!(
            view.momentum_per_orbit.unwrap().si,
            rw.secular_per_orbit.value()
        );
        assert_eq!(view.unloads_per_day.map(|q| q.si), rw.unloads_per_day);
        assert_eq!(
            view.unload_times
                .iter()
                .map(|u| (u.means, u.time.si))
                .collect::<Vec<_>>(),
            [(
                crate::dto::AocsUnloadMeans::Mtq,
                unload.mtq.unwrap().time.value()
            )]
        );
        assert_eq!(
            view.sensor_fov.verdict,
            Verdict::worst(fov.checks.iter().map(|c| c.verdict))
        );
        momenta.push(rw.secular_per_orbit.value());
    }
    assert!(momenta.windows(2).any(|w| w[0] != w[1]));
}

#[test]
fn default_projection_is_identical_to_explicit_default_mode() {
    let s = demo().snapshot().unwrap();
    let ctx = fune_projection::Context::new(&s.design, &s.resolved, Some("hot_imaging")).unwrap();
    let mode = fune_projection::timeline::case_mode_id(&ctx);
    assert_eq!(
        serde_json::to_value(fune_projection::aocs::rw_unloading(&ctx).unwrap()).unwrap(),
        serde_json::to_value(
            fune_projection::aocs::rw_unloading_for_mode(&ctx, mode.as_deref()).unwrap()
        )
        .unwrap()
    );
    assert_eq!(
        serde_json::to_value(fune_projection::aocs::sensor_fov(&ctx).unwrap()).unwrap(),
        serde_json::to_value(
            fune_projection::aocs::sensor_fov_for_mode(&ctx, mode.as_deref()).unwrap()
        )
        .unwrap()
    );
}

#[test]
fn cards_handle_missing_law_and_failed_analysis_independently() {
    let mut s = demo().snapshot().unwrap();
    s.design.analysis.aocs.as_mut().unwrap().law.remove("SAFE");
    let view = aocs::mode_view(&s, "SAFE").unwrap();
    assert!(view.attitude.law.is_none());
    assert_eq!(view.rw.status, AnalysisRunStatus::MissingInputs);
    assert_eq!(view.sensor_fov.status, AnalysisRunStatus::MissingInputs);
    assert!(view.momentum_per_orbit.is_none());
    assert!(view.sensor_fov.verdict.is_none());
    assert!(!view.devices.is_empty());
    assert!(
        view.rw
            .missing
            .iter()
            .any(|m| m.path == "analysis.aocs.law.SAFE")
    );
    s.design
        .analysis
        .aocs
        .as_mut()
        .unwrap()
        .usable_momentum_fraction = Some(2.0);
    let view = aocs::mode_view(&s, "NOMINAL").unwrap();
    assert_eq!(view.rw.status, AnalysisRunStatus::Failed);
    assert!(view.rw.error.is_some());
    assert!(view.momentum_per_orbit.is_none());
    assert_eq!(view.sensor_fov.status, AnalysisRunStatus::Ok);
}

#[test]
fn counted_devices_include_duty_cold_spares_off_and_unassigned() {
    let mut s = demo().snapshot().unwrap();
    let part = s.design.parts.get_mut("RW_X").unwrap();
    part.count = 3;
    part.redundancy = fune_model::design::Redundancy::Cold;
    let assignment = s
        .design
        .modes
        .get_mut("NOMINAL")
        .unwrap()
        .states
        .get_mut("RW_X")
        .unwrap();
    assignment.state = "on".into();
    assignment.active = Some(2);
    assignment.duty = [("peak".into(), 0.25)].into();
    let s = Snapshot::of(s.design, Default::default());
    let view = aocs::mode_view(&s, "NOMINAL").unwrap();
    let group = view
        .devices
        .iter()
        .find(|g| g.role == Some(Role::ReactionWheel))
        .unwrap();
    let wheel = group.devices.iter().find(|d| d.part == "RW_X").unwrap();
    assert_eq!(wheel.active, Some(true));
    assert!(
        wheel
            .states
            .iter()
            .any(|s| s.state == "peak" && s.count == 2 && s.fraction.si == 0.25)
    );
    assert!(
        wheel
            .states
            .iter()
            .any(|s| s.state == STATE_OFF && s.count == 1 && s.fraction.si == 1.0)
    );
    // RW_Y と RW_Z の各 1 台 + RW_X の稼働 2 台（予備 1 台は含めない）。
    assert_eq!((group.active_count, group.total_count), (4, 5));
}

#[test]
fn unassigned_devices_are_unknown() {
    let mut design = demo().snapshot().unwrap().design;
    design
        .modes
        .get_mut("NOMINAL")
        .unwrap()
        .states
        .remove("RW_X");
    let s = Snapshot::of(design, Default::default());
    let view = aocs::mode_view(&s, "NOMINAL").unwrap();
    assert_eq!(
        view.devices
            .iter()
            .find(|g| g.role == Some(Role::ReactionWheel))
            .unwrap()
            .devices
            .iter()
            .find(|d| d.part == "RW_X")
            .unwrap()
            .active,
        None
    );
}

#[test]
fn all_attitude_fields_round_trip_through_the_existing_set_text_entry() {
    let ws = demo();
    let s = ws.snapshot().unwrap();
    for mode in s.design.ordered_modes() {
        let view = aocs::mode_view(&s, mode.id.as_str()).unwrap();
        for field in view.attitude.fields {
            let op = crate::edit::set_op(
                &ws,
                &SetText {
                    path: field.path,
                    value: field.text,
                    ..Default::default()
                },
            )
            .unwrap();
            let updated = crate::preview::snapshot_with(&ws, &[op]).unwrap();
            let next = aocs::mode_view(&updated, mode.id.as_str()).unwrap();
            assert_eq!(
                serde_json::to_value(view.attitude.axes.clone()).unwrap(),
                serde_json::to_value(next.attitude.axes).unwrap()
            );
        }
    }
}

#[test]
fn editing_law_axes_and_inertial_orientation_recomputes_the_card() {
    let ws = demo();
    let mut ops = Vec::new();
    for (field, value) in [
        ("law", "nadir_pointing"),
        ("axis", "0 1 0"),
        ("along_track", "1 0 0"),
    ] {
        ops.push(
            crate::edit::set_op(
                &ws,
                &SetText {
                    path: format!("analysis.aocs.law.SAFE.{field}"),
                    value: value.into(),
                    ops: ops.clone(),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
    }
    let s = crate::preview::snapshot_with(&ws, &ops).unwrap();
    let view = aocs::mode_view(&s, "SAFE").unwrap();
    assert_eq!(
        view.attitude.axes[0].body,
        format!("[0, {}, 0]", crate::units::number_text(1.0))
    );
    assert_eq!(view.attitude.axes[0].target_en, "Nadir");
    assert_eq!(view.rw.status, AnalysisRunStatus::Ok);
    for (field, value) in [("law", "inertial"), ("orientation", "0 1 0 0")] {
        ops.push(
            crate::edit::set_op(
                &ws,
                &SetText {
                    path: format!("analysis.aocs.law.SAFE.{field}"),
                    value: value.into(),
                    ops: ops.clone(),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
    }
    let s = crate::preview::snapshot_with(&ws, &ops).unwrap();
    let view = aocs::mode_view(&s, "SAFE").unwrap();
    assert_eq!(
        view.attitude.axes[0].target_en,
        format!("ECI [0, 0, {}]", crate::units::number_text(-1.0))
    );
    assert_eq!(view.rw.status, AnalysisRunStatus::Ok);
    // プレビューは保存済みの姿勢則を変えない。
    assert_eq!(
        ws.snapshot().unwrap().design.analysis.aocs.unwrap().law["SAFE"].law,
        fune_model::design::AttitudeLawKind::SunPointing
    );
}

#[test]
fn eclipse_devices_use_the_inherited_and_overridden_states() {
    let mut s = demo().snapshot().unwrap();
    let mode = s.design.modes.get_mut("NOMINAL").unwrap();
    let mut assignment = mode.states["RW_X"].clone();
    assignment.state = STATE_OFF.into();
    assignment.duty.clear();
    mode.eclipse.insert("RW_X".into(), assignment);
    let s = Snapshot::of(s.design, Default::default());
    let view = aocs::mode_view(&s, "NOMINAL").unwrap();
    let group = |groups: &[crate::dto::AocsDeviceGroup]| {
        groups
            .iter()
            .find(|g| g.role == Some(Role::ReactionWheel))
            .map(|g| g.active_count)
            .unwrap()
    };
    assert_eq!(group(&view.devices), 3);
    assert_eq!(group(&view.eclipse_devices), 2);
}

/// 機器の一覧は AOCS の系に属する部品だけを役割別に並べる。ペイロードのカメラは出ず、役割の無い種別の部品は「その他」に入る。
#[test]
fn devices_are_the_parts_of_the_aocs_system_grouped_by_role_with_other_last() {
    use fune_model::design::Part;
    use fune_model::id::Id;
    let mut d = demo().snapshot().unwrap().design;
    let mut kind = fune_model::builtin::kinds()["gps"].clone();
    kind.id = Id::new("sun_lidar");
    kind.role = None;
    kind.states = vec!["off".into(), "on".into()];
    d.kinds.insert(kind.id.clone(), kind);
    let mut part: Part = d.parts["GPS"].clone();
    part.id = Id::new("LIDAR");
    part.catalog = None;
    part.kind = Some(Id::new("sun_lidar"));
    d.parts.insert(part.id.clone(), part);
    let s = Snapshot::of(d, Default::default());
    let view = aocs::mode_view(&s, "NOMINAL").unwrap();
    let roles: Vec<Option<Role>> = view.devices.iter().map(|g| g.role).collect();
    assert_eq!(
        roles,
        [
            Some(Role::StarTracker),
            Some(Role::SunSensor),
            Some(Role::Gps),
            Some(Role::ReactionWheel),
            Some(Role::Magnetorquer),
            None,
        ]
    );
    let all: Vec<&str> = view
        .devices
        .iter()
        .flat_map(|g| g.devices.iter().map(|d| d.part.as_str()))
        .collect();
    assert!(!all.contains(&"CAM"), "{all:?}");
    let other = view.devices.last().unwrap();
    assert_eq!(
        (other.label.as_str(), other.label_en.as_deref()),
        ("その他", Some("Other"))
    );
    assert_eq!(
        other
            .devices
            .iter()
            .map(|d| d.part.as_str())
            .collect::<Vec<_>>(),
        ["LIDAR"]
    );
    // 見出しは役割の名前（`role_label`）
    let gps = &view.devices[2];
    assert_eq!(
        gps.label,
        fune_model::builtin::role_label(Role::Gps).unwrap().label
    );
    assert_eq!(view.eclipse_devices.len(), view.devices.len());
}

/// スラスタだけでアンロードする設計では、モードカードにも RCS の時間が出る（MTQ は出ない）。
#[test]
fn rcs_only_unloading_shows_on_the_card() {
    let o: sat_analysis::aocs::UnloadTimeOutput = serde_json::from_value(
        crate::analysis::kpi::tests::unload_output(None, Some(Some(40.0))),
    )
    .unwrap();
    let times = aocs::unload_times(&o);
    assert_eq!(times.len(), 1);
    assert_eq!(times[0].means, crate::dto::AocsUnloadMeans::Rcs);
    assert_eq!(times[0].time.si, 40.0);
    assert_eq!(times[0].label, "RCS（最悪の向き）");
}
