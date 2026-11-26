//! 地上局（`conditions/ground/<ID>.toml`）の位置と仰角の範囲の検査。単位と次元は設定の次元の表（`Scope::dimensions`）が見るので、
//! ここは単位が読めて角度である値の大きさだけを見る。経度は下流（局の位置ベクトル・パスの計算）が三角関数にしか使わないので、
//! 東経を 0〜360° で書く流儀も受ける（-180〜360°。値は書き換えない）。

use super::Ctx;
use super::settings::{Scope, scope};
use crate::design::{GroundStationSettings, SettingEntry};
use crate::path::SettingPath;
use crate::value::Quantity;

/// 浮動小数の丸めを許す幅（度）。
const EPS_DEG: f64 = 1e-9;

/// 角度の量を度にする。読めない・角度でない量は `None`（単位の誤りは次元の検査が報告する）。
fn degrees(q: &Quantity) -> Option<f64> {
    q.si_as("angle").ok().map(f64::to_degrees)
}

/// `conditions/ground/` の全体（量の次元と、緯度・経度・最低仰角の範囲）。`entries` は `Design::setting_entries`。
pub(super) fn check_ground_stations(ctx: &mut Ctx, entries: &[SettingEntry]) {
    let design = ctx.design;
    for (id, station) in &design.conditions.ground {
        let mut s = scope(ctx, SettingPath::conditions(&["ground", id]));
        s.dimensions(entries);
        check_ground(&mut s, station);
    }
}

/// 緯度・経度・最低仰角の範囲を検査する。
fn check_ground(s: &mut Scope, g: &GroundStationSettings) {
    let ranges = [
        (
            "latitude",
            &g.latitude,
            -90.0,
            90.0,
            "Latitude must be from -90 to 90 deg",
        ),
        (
            "longitude",
            &g.longitude,
            -180.0,
            360.0,
            "Longitude must be from -180 to 360 deg (east longitude may be written as 0 to 360)",
        ),
        (
            "min_elevation",
            &g.min_elevation,
            0.0,
            90.0,
            "The minimum elevation must be from 0 to 90 deg",
        ),
    ];
    for (key, quantity, low, high, message) in ranges {
        let out = quantity
            .as_ref()
            .and_then(degrees)
            .is_some_and(|d| !(d.is_finite() && d >= low - EPS_DEG && d <= high + EPS_DEG));
        if out {
            s.error(&[key], "setting.range", message.to_owned());
        }
    }
}
