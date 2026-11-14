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
}

impl Scope<'_, '_> {
    /// ファイルの中のキーの列 → 設定の対象。
    pub fn path(&self, keys: &[&str]) -> Option<Target> {
        Some(self.prefix.child(keys).into())
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
                let target = Some(entry.path.clone().into());
                self.ctx.report(sev, code, &self.entity, &keys, target, msg);
            }
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

/// 設定ファイル（`conditions.orbit` など）の検査の窓口を作る。
pub(super) fn scope<'c, 'a>(ctx: &'c mut Ctx<'a>, prefix: SettingPath) -> Scope<'c, 'a> {
    Scope {
        ctx,
        entity: EntityKey::Setting(prefix.clone()),
        prefix,
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

fn check_orbit(s: &mut Scope, o: &OrbitSettings) {
    let non_earth = o
        .central
        .is_some_and(|c| c != crate::design::CentralBodyKind::Earth);
    if non_earth && o.sun_synchronous == Some(true) {
        s.error(
            &["sun_synchronous"],
            "setting.central",
            "太陽同期の傾斜角は地球周回だけで求めます（inclination を書きます）".into(),
        );
    }
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
                format!("材料 `{m}` が analysis/structure.toml の [material.*] にありません"),
            );
        }
    }
}
