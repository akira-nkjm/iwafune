//! `conditions/` の設定と材料の参照の検査。`analysis/` は [`super::settings_analysis`]。
//! 共通の道具（設定ファイル 1 つ分の検査の窓口 [`Scope`]）もここに置く。
//! 量の次元は設定の次元の表（`design::setting_dims`）を引く（ファイルの中の量の葉をすべて見る。[`Scope::dimensions`]）。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::setting_dims::setting_dimension;
use crate::design::{EntityKey, OrbitSettings, SettingEntry};
use crate::path::SettingPath;
use crate::target::Target;
use crate::value::{Quantity, SettingValue, Value};

/// 設定ファイル 1 つ分の検査の窓口。
pub(super) struct Scope<'c, 'a> {
    pub ctx: &'c mut Ctx<'a>,
    pub entity: EntityKey,
    /// 設定ファイルのパス（`conditions.orbit` など）。キーの列を足して対象にする
    pub prefix: SettingPath,
    /// フェーズのファイルの中の設定（フェーズの ID と、ファイルの中の置き場所のキー。`phases/LEOP.toml` の `[orbit]` なら
    /// `("LEOP", ["orbit"])`）。あれば、診断の対象と位置はフェーズのファイルのものにする（`prefix` は表の形の引き当てだけに使う）
    pub phase: Option<(String, Vec<String>)>,
}

impl Scope<'_, '_> {
    /// ファイルの中のキーの列 → 設定の対象。
    pub fn path(&self, keys: &[&str]) -> Option<Target> {
        Some(match &self.phase {
            Some((id, base)) => {
                let all: Vec<&str> = base
                    .iter()
                    .map(String::as_str)
                    .chain(keys.iter().copied())
                    .collect();
                Target::phase(id.as_str(), Some(&all.join(".")))
            }
            None => self.prefix.child(keys).into(),
        })
    }

    /// 設定のキーの列 → 位置を引くファイルの中のキーの列（フェーズなら置き場所のキーを前に足す）。
    fn file_keys(&self, keys: &[&str]) -> Vec<String> {
        let base = self.phase.iter().flat_map(|(_, b)| b.iter().cloned());
        base.chain(keys.iter().map(|k| (*k).to_owned())).collect()
    }

    /// 診断を足す（位置と対象の組み立てはここだけ）。
    fn report(&mut self, sev: crate::diagnostic::Severity, code: &str, keys: &[&str], msg: String) {
        let path = self.path(keys);
        let file_keys = self.file_keys(keys);
        let refs: Vec<&str> = file_keys.iter().map(String::as_str).collect();
        self.ctx.report(sev, code, &self.entity, &refs, path, msg);
    }

    /// このファイルの量の葉（ベクトルは成分の単位）の次元を、設定の次元の表と比べる。`entries` は `Design::setting_entries`。
    pub fn dimensions(&mut self, entries: &[SettingEntry]) {
        for entry in entries {
            let (scope, segments) = (entry.path.scope, &entry.path.segments);
            if scope != self.prefix.scope || !segments.starts_with(&self.prefix.segments) {
                continue;
            }
            let q = match &entry.value {
                Some(SettingValue::Quantity(q)) => q.clone(),
                Some(SettingValue::Vector(v)) => {
                    Quantity::new(v.x, v.unit.clone().unwrap_or_default())
                }
                _ => continue,
            };
            let Some(dim) = setting_dimension(&entry.path) else {
                continue;
            };
            if let Some((sev, code, msg)) = check_quantity_dimension(&q, dim) {
                let keys: Vec<&str> = segments[self.prefix.segments.len()..]
                    .iter()
                    .map(String::as_str)
                    .collect();
                self.report(sev, code, &keys, msg);
            }
        }
    }

    pub fn fraction(&mut self, keys: &[&str], v: Option<f64>) {
        if v.is_some_and(|v| !(0.0..=1.0).contains(&v)) {
            self.error(keys, "setting.range", "Must be a value from 0 to 1".into());
        }
    }

    pub fn error(&mut self, keys: &[&str], code: &str, message: String) {
        self.report(crate::diagnostic::Severity::Error, code, keys, message);
    }

    pub fn warning(&mut self, keys: &[&str], code: &str, message: String) {
        self.report(crate::diagnostic::Severity::Warning, code, keys, message);
    }

    pub fn reference(&mut self, keys: &[&str], exists: bool, code: &str, what: String) {
        if !exists {
            self.error(keys, code, format!("{what} is not defined"));
        }
    }
}

/// 設定ファイル（`conditions.orbit` など）の検査の窓口を作る。
pub(super) fn scope<'c, 'a>(ctx: &'c mut Ctx<'a>, prefix: SettingPath) -> Scope<'c, 'a> {
    Scope {
        ctx,
        entity: EntityKey::Setting(prefix.clone()),
        prefix,
        phase: None,
    }
}

/// フェーズのファイルの中の軌道（`[orbit]`）の検査の窓口。診断の対象と位置はフェーズのファイルのもの。
pub(super) fn phase_orbit_scope<'c, 'a>(ctx: &'c mut Ctx<'a>, phase: &str) -> Scope<'c, 'a> {
    Scope {
        ctx,
        entity: EntityKey::Phase(phase.to_owned()),
        prefix: SettingPath::conditions(&["orbit"]),
        phase: Some((phase.to_owned(), vec!["orbit".to_owned()])),
    }
}

/// `conditions/` の全体。`entries` は `Design::setting_entries`。
pub(super) fn check_conditions(ctx: &mut Ctx, entries: &[SettingEntry]) {
    let design = ctx.design;
    if let Some(o) = &design.conditions.orbit {
        let mut s = scope(ctx, SettingPath::conditions(&["orbit"]));
        s.dimensions(entries);
        check_orbit(&mut s, o);
    }
    for id in design.conditions.ground.keys() {
        scope(ctx, SettingPath::conditions(&["ground", id])).dimensions(entries);
    }
    if design.conditions.launch.is_some() {
        scope(ctx, SettingPath::conditions(&["launch"])).dimensions(entries);
    }
}

/// 軌道の設定の検査（設計の `conditions/orbit.toml` とフェーズの `[orbit]` で同じ）。
pub(super) fn check_orbit(s: &mut Scope, o: &OrbitSettings) {
    let non_earth = o
        .central
        .is_some_and(|c| c != crate::design::CentralBodyKind::Earth);
    if non_earth && o.sun_synchronous == Some(true) {
        s.error(
            &["sun_synchronous"],
            "setting.central",
            "The sun-synchronous inclination is computed for Earth orbits only. Set inclination instead".into(),
        );
    }
    if o.propagator == Some(crate::design::PropagatorKind::Numerical) {
        s.warning(
            &["propagator"],
            "setting.not_applied",
            "propagator = \"numerical\" is not used by the analyses. There is no default force model, so time-series analyses propagate with the J2 secular terms (j2). \
             Choose kepler or j2 to analyze with that propagation (numerical propagation reads the elements as osculating elements, so the same numbers give a different orbit than j2)"
                .into(),
        );
    }
    if o.eccentricity.is_some_and(|e| !(0.0..1.0).contains(&e)) {
        s.error(
            &["eccentricity"],
            "setting.range",
            "Eccentricity must be at least 0 and less than 1".into(),
        );
    }
    if o.ltdn
        .as_deref()
        .is_some_and(|t| crate::value::parse_clock_hours(t).is_none())
    {
        s.error(
            &["ltdn"],
            "setting.format",
            "The local time of the descending node must be written as `HH:MM`".into(),
        );
    }
    // 以前の書き方（軌道維持の推薬を `[decay]` に書く）は壊さずに警告して、推進系へ移すよう促す
    if o.decay.as_ref().is_some_and(|d| d.propellant.is_some()) {
        s.warning(
            &["decay", "propellant"],
            "setting.moved",
            "Orbit-keeping propellant uses the propellants available to the propulsion analysis (propulsion.propellant), so this value is not used. Set the propellant with propulsion parts (propellant_tank, propulsion_thruster) and maneuvers (maneuvers/), then remove it with `fune setting unset conditions.orbit.decay.propellant`".into(),
        );
    }
}

/// 部品・カタログ品の `material` が `analysis/structure.toml` の材料にあるか。
pub(super) fn check_materials(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(structure) = &design.analysis.structure else {
        return;
    };
    let parts = design.parts.values().map(|p| {
        let target = Target::attr(p.id.as_str(), "material");
        (EntityKey::Part(p.id.to_string()), target, &p.attr)
    });
    let items = design.catalog.values().map(|c| {
        let target = Target::Catalog {
            id: c.id.clone(),
            attr: Some(crate::path::AttrPath {
                attr: "material".to_owned(),
                state: None,
            }),
        };
        (EntityKey::Catalog(c.id.clone()), target, &c.attr)
    });
    for (entity, target, attrs) in parts.chain(items) {
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
                Some(target),
                format!("Material `{m}` is not in [material.*] of analysis/structure.toml"),
            );
        }
    }
}
