//! `mission/` の設定と材料の参照の検査。`analysis/` は [`super::settings_analysis`]。
//! 共通の道具（設定ファイル 1 つ分の検査の窓口 [`Scope`]）もここに置く。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::{EntityKey, GroundStationSettings, OrbitSettings};
use crate::value::{Quantity, Value, Vector3};

/// 設定ファイル 1 つ分の検査の窓口。
pub(super) struct Scope<'c, 'a> {
    pub ctx: &'c mut Ctx<'a>,
    pub entity: EntityKey,
    pub prefix: String,
}

/// 量の検査の 1 行（キーの列・量・次元）。
pub(super) type QtySpec<'q> = (&'q [&'q str], Option<&'q Quantity>, &'q str);

impl Scope<'_, '_> {
    pub fn path(&self, keys: &[&str]) -> Option<String> {
        Some(format!("{}.{}", self.prefix, keys.join(".")))
    }

    pub fn qty(&mut self, keys: &[&str], q: Option<&Quantity>, dim: &str) {
        if let Some((sev, code, msg)) = q.and_then(|q| check_quantity_dimension(q, dim)) {
            let path = self.path(keys);
            self.ctx.report(sev, code, &self.entity, keys, path, msg);
        }
    }

    pub fn qtys(&mut self, specs: &[QtySpec]) {
        for (keys, q, dim) in specs {
            self.qty(keys, *q, dim);
        }
    }

    pub fn vector(&mut self, keys: &[&str], v: Option<&Vector3>, dim: &str) {
        if let Some(v) = v {
            let q = Quantity::new(v.x, v.unit.clone().unwrap_or_default());
            self.qty(keys, Some(&q), dim);
        }
    }

    pub fn fraction(&mut self, keys: &[&str], v: Option<f64>) {
        if v.is_some_and(|v| !(0.0..=1.0).contains(&v)) {
            self.error(keys, "setting.range", "0〜1 の値にします".into());
        }
    }

    pub fn error(&mut self, keys: &[&str], code: &str, message: String) {
        let path = self.path(keys);
        self.ctx.error(code, &self.entity, keys, path, message);
    }

    pub fn warning(&mut self, keys: &[&str], code: &str, message: String) {
        let path = self.path(keys);
        self.ctx.warning(code, &self.entity, keys, path, message);
    }

    pub fn reference(&mut self, keys: &[&str], exists: bool, code: &str, what: String) {
        if !exists {
            self.error(keys, code, format!("{what} がありません"));
        }
    }
}

/// 設定ファイル（`"mission.orbit"` など）の検査の窓口を作る。
pub(super) fn scope<'c, 'a>(ctx: &'c mut Ctx<'a>, name: &str) -> Scope<'c, 'a> {
    Scope {
        ctx,
        entity: EntityKey::Setting(name.to_string()),
        prefix: name.to_string(),
    }
}

/// `mission/` の全体。
pub(super) fn check_mission(ctx: &mut Ctx) {
    let design = ctx.design;
    if let Some(o) = &design.mission.orbit {
        check_orbit(&mut scope(ctx, "mission.orbit"), o);
    }
    for (id, g) in &design.mission.ground {
        check_ground(&mut scope(ctx, &format!("mission.ground.{id}")), g);
    }
    if let Some(l) = &design.mission.launch {
        scope(ctx, "mission.launch").qtys(&[
            (&["load_axial"], l.load_axial.as_ref(), "Acceleration"),
            (&["load_lateral"], l.load_lateral.as_ref(), "Acceleration"),
            (
                &["min_frequency_axial"],
                l.min_frequency_axial.as_ref(),
                "Frequency",
            ),
            (
                &["min_frequency_lateral"],
                l.min_frequency_lateral.as_ref(),
                "Frequency",
            ),
        ]);
    }
}

fn check_orbit(s: &mut Scope, o: &OrbitSettings) {
    s.qtys(&[
        (&["altitude"], o.altitude.as_ref(), "Length"),
        (&["semi_major_axis"], o.semi_major_axis.as_ref(), "Length"),
        (&["inclination"], o.inclination.as_ref(), "Angle"),
        (&["raan"], o.raan.as_ref(), "Angle"),
        (&["arg_perigee"], o.arg_perigee.as_ref(), "Angle"),
        (&["mean_anomaly"], o.mean_anomaly.as_ref(), "Angle"),
    ]);
    if o.eccentricity.is_some_and(|e| !(0.0..1.0).contains(&e)) {
        s.error(
            &["eccentricity"],
            "setting.range",
            "離心率は 0 以上 1 未満にします".into(),
        );
    }
    if o.ltdn
        .as_deref()
        .is_some_and(|t| crate::value::parse_clock_hours(t).is_none())
    {
        s.error(
            &["ltdn"],
            "setting.format",
            "降交点の地方時は `HH:MM` で書きます".into(),
        );
    }
}

fn check_ground(s: &mut Scope, g: &GroundStationSettings) {
    s.qtys(&[
        (&["latitude"], g.latitude.as_ref(), "Angle"),
        (&["longitude"], g.longitude.as_ref(), "Angle"),
        (&["min_elevation"], g.min_elevation.as_ref(), "Angle"),
        (&["altitude"], g.altitude.as_ref(), "Length"),
    ]);
    for (band, link) in &g.links {
        let b = band.as_str();
        s.qtys(&[
            (
                &["link", b, "antenna_gain"],
                link.antenna_gain.as_ref(),
                "Dimensionless",
            ),
            (
                &["link", b, "noise_temperature"],
                link.noise_temperature.as_ref(),
                "Temperature",
            ),
            (
                &["link", b, "g_over_t"],
                link.g_over_t.as_ref(),
                "Dimensionless",
            ),
            (&["link", b, "tx_power"], link.tx_power.as_ref(), "Power"),
            (
                &["link", b, "line_loss"],
                link.line_loss.as_ref(),
                "Dimensionless",
            ),
        ]);
    }
}

/// 部品・カタログ品の `material` が `analysis/structure.toml` の材料にあるか。
pub(super) fn check_materials(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(structure) = &design.analysis.structure else {
        return;
    };
    let parts = design
        .parts
        .values()
        .map(|p| (EntityKey::Part(p.id.to_string()), p.id.to_string(), &p.attr));
    let items = design
        .catalog
        .values()
        .map(|c| (EntityKey::Catalog(c.id.clone()), c.id.clone(), &c.attr));
    for (entity, owner, attrs) in parts.chain(items) {
        let Some(value) = attrs.get("material") else {
            continue;
        };
        if let Value::Enum(m) | Value::Text(m) = &value.value
            && !structure.material.contains_key(m)
        {
            ctx.warning(
                "ref.material",
                &entity,
                &["attr", "material"],
                Some(format!("{owner}.material")),
                format!("材料 `{m}` が analysis/structure.toml の [material.*] にありません"),
            );
        }
    }
}
