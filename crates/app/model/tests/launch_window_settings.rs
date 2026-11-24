//! `analysis/propulsion.toml` の `[launch_window]` の検査: 格子の点の数は軸ごとに 2 以上、時間の量は時間の次元。

use fune_model::Design;
use fune_model::design::{PropulsionLaunchWindow, PropulsionSettings};
use fune_model::value::Quantity;

fn design(w: PropulsionLaunchWindow) -> Design {
    let mut d = Design::default();
    d.analysis.propulsion = Some(PropulsionSettings {
        launch_window: Some(w),
        ..PropulsionSettings::default()
    });
    d
}

fn range_errors(w: PropulsionLaunchWindow) -> usize {
    fune_model::check(&design(w))
        .iter()
        .filter(|d| d.code == "setting.range")
        .count()
}

#[test]
fn grid_steps_must_be_at_least_two() {
    assert_eq!(range_errors(PropulsionLaunchWindow::default()), 0);
    let steps = |d, a| PropulsionLaunchWindow {
        departure_steps: d,
        arrival_steps: a,
        ..PropulsionLaunchWindow::default()
    };
    assert_eq!(range_errors(steps(Some(2), Some(100))), 0);
    assert_eq!(range_errors(steps(Some(1), None)), 1);
    assert_eq!(range_errors(steps(Some(0), Some(1))), 2);
}

#[test]
fn time_settings_need_a_time_dimension() {
    let w = PropulsionLaunchWindow {
        departure_span: Some(Quantity::new(400.0, "d")),
        time_of_flight_min: Some(Quantity::new(150.0, "d")),
        ..PropulsionLaunchWindow::default()
    };
    let wrong = |w: &PropulsionLaunchWindow| {
        fune_model::check(&design(w.clone()))
            .iter()
            .filter(|d| d.code == "unit.dimension")
            .count()
    };
    assert_eq!(wrong(&w), 0);
    let bad = PropulsionLaunchWindow {
        time_of_flight_max: Some(Quantity::new(3.0, "kg")),
        ..w
    };
    assert!(wrong(&bad) > 0);
}
