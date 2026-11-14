//! プロジェクト独自の属性定義・種別定義の検査（ID の規則とフィールド名との重なり、次元と許可単位の一致、参照、継承の循環）。

use super::Ctx;
use crate::builtin;
use crate::design::{AttributeDef, EntityKey, KindDef};
use crate::field_names::{CatalogFieldName, PartFieldName};
use crate::id::{clash, validate_local_name};
use crate::target::Target;
use crate::value::ValueType;
use crate::value::units::{expect_dimension, parse_dimension_name, same_dimension, unit_info};

/// 属性定義だけを検査する（CreateAttribute の事前検査にも使う）。問題の本文を返す。
pub fn attribute_def_problems(def: &AttributeDef) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if let Err(e) = validate_local_name(def.id.as_str()) {
        out.push(("id.invalid", e.to_string()));
    }
    if let Some(message) = field_name_clash(def.id.as_str()) {
        out.push(("def.reserved", message));
    }
    let needs_unit = matches!(
        def.value_type,
        ValueType::Scalar | ValueType::Vec3 | ValueType::Range | ValueType::Tensor3
    );
    if !needs_unit && (def.dimension.is_some() || !def.units.is_empty()) {
        out.push((
            "def.dimension",
            format!("型 {:?} の属性に次元・単位は書けません", def.value_type),
        ));
    }
    if def.by_state && def.value_type != ValueType::Scalar {
        out.push(("def.by_state", "状態ごとの値はスカラーだけです".to_string()));
    }
    match &def.dimension {
        Some(dim) => {
            if parse_dimension_name(dim).is_err() {
                out.push(("def.dimension", format!("次元名 `{dim}` を知りません")));
            } else {
                for unit in def.units.iter().chain(def.display_unit.iter()) {
                    if let Err(e) = expect_dimension(unit, dim) {
                        out.push(("def.unit", e.to_string()));
                    }
                }
            }
        }
        None => {
            let first = def.units.first();
            for unit in def.units.iter().chain(def.display_unit.iter()) {
                match (unit_info(unit), first.map(|f| same_dimension(unit, f))) {
                    (Err(e), _) => out.push(("def.unit", e.to_string())),
                    (Ok(_), Some(Ok(false))) => out.push((
                        "def.unit",
                        format!("許可単位 `{unit}` の次元が他の単位とそろっていません"),
                    )),
                    _ => {}
                }
            }
            if needs_unit && def.units.is_empty() {
                out.push((
                    "def.dimension",
                    "次元（dimension）か許可単位（units）を書いてください".to_string(),
                ));
            }
        }
    }
    out
}

/// 属性 ID が部品・カタログ品のフィールド名（[`PartFieldName`]・[`CatalogFieldName`]）と重なるなら、その問題の本文。
/// パスの記法（`RW_X.label`・`fune catalog set <品> label`）では属性とフィールドを区別できないため。ID の一意性と同じく
/// 大文字小文字は無視する。
fn field_name_clash(id: &str) -> Option<String> {
    let owners: Vec<&str> = [
        (PartFieldName::NAMES, "部品"),
        (CatalogFieldName::NAMES, "カタログ品"),
    ]
    .into_iter()
    .filter(|(fields, _)| clash(fields.iter().copied(), id).is_some())
    .map(|(_, owner)| owner)
    .collect();
    (!owners.is_empty()).then(|| {
        format!(
            "属性 ID `{id}` は{}のフィールド名と同じです（パスの記法で属性とフィールドを区別できません）",
            owners.join("・")
        )
    })
}

/// プロジェクト独自の定義。
pub(super) fn check_defs(ctx: &mut Ctx) {
    let design = ctx.design;
    for def in design.attributes.values() {
        let entity = EntityKey::Attribute(def.id.to_string());
        let target = || {
            Some(Target::AttributeDef {
                id: def.id.to_string(),
            })
        };
        if builtin::is_builtin_attribute(def.id.as_str()) {
            ctx.error(
                "def.shadow_builtin",
                &entity,
                &[],
                target(),
                format!("属性 `{}` は組み込みの定義と同じ ID です", def.id),
            );
        }
        for (code, message) in attribute_def_problems(def) {
            ctx.error(code, &entity, &[], target(), message);
        }
    }
    for kind in design.kinds.values() {
        check_kind(ctx, kind);
    }
}

/// プロジェクト独自の種別 1 つ（組み込みとの重複・継承元・循環・属性の参照）。
fn check_kind(ctx: &mut Ctx, kind: &KindDef) {
    let design = ctx.design;
    let entity = EntityKey::Kind(kind.id.to_string());
    let path = Some(Target::KindDef {
        id: kind.id.to_string(),
    });
    if builtin::is_builtin_kind(kind.id.as_str()) {
        ctx.error(
            "def.shadow_builtin",
            &entity,
            &[],
            path.clone(),
            format!("種別 `{}` は組み込みの定義と同じ ID です", kind.id),
        );
    }
    if let Some(parent) = &kind.extends
        && design.kind(parent.as_str()).is_none()
    {
        ctx.error(
            "ref.kind",
            &entity,
            &["extends"],
            path.clone(),
            format!("継承元の種別 `{parent}` がありません"),
        );
    }
    let chain = design.kind_chain(kind.id.as_str());
    if chain
        .last()
        .and_then(|last| last.extends.as_ref())
        .is_some_and(|e| design.kind(e.as_str()).is_some())
    {
        ctx.error(
            "kind.cycle",
            &entity,
            &["extends"],
            path.clone(),
            "種別の継承が循環しています",
        );
    }
    for attr in kind.required.iter().chain(&kind.optional) {
        if design.attribute(attr.as_str()).is_none() {
            ctx.error(
                "ref.attribute",
                &entity,
                &[],
                path.clone(),
                format!("属性 `{attr}` の定義がありません"),
            );
        }
    }
    check_thruster_firing_state(ctx, kind, &entity, path);
}

/// スラスタの役割（thruster・propulsion_thruster）の種別の語彙に噴射の状態 [`builtin::STATE_THRUSTER_FIRING`] が無ければ warning。
/// 噴射中の電力と推力誤差の擾乱はこの綴りの状態から読むので、無いと黙って 0 になる。
fn check_thruster_firing_state(
    ctx: &mut Ctx,
    kind: &KindDef,
    entity: &EntityKey,
    path: Option<Target>,
) {
    let design = ctx.design;
    let id = kind.id.as_str();
    let is_thruster = matches!(
        design.kind_role(id),
        Some(crate::role::Role::Thruster | crate::role::Role::PropulsionThruster)
    );
    let states = design.kind_states(id);
    if is_thruster && !states.iter().any(|s| s == builtin::STATE_THRUSTER_FIRING) {
        ctx.warning(
            "def.thruster_firing_state",
            entity,
            &["states"],
            path,
            format!(
                "スラスタの種別 `{id}` の状態に `{}` がありません。噴射中の電力と推力誤差の擾乱は状態 `{}` から読むので、このままでは 0 になります",
                builtin::STATE_THRUSTER_FIRING,
                builtin::STATE_THRUSTER_FIRING
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::Design;
    use crate::diagnostic::Severity;
    use crate::id::Id;

    /// 組み込みの `heat_fraction`（無次元のスカラー）を別の ID で定義し直したもの。
    fn def(id: &str) -> AttributeDef {
        let mut def = builtin::attributes()["heat_fraction"].clone();
        def.id = Id::new(id);
        def
    }

    /// 部品・カタログ品のフィールド名と同じ属性 ID は error（対象は属性定義）。大文字小文字を変えても同じ。
    #[test]
    fn attribute_ids_must_not_be_field_names() {
        let mut design = Design::default();
        for id in ["label", "count", "part_number", "Maturity", "tank_pressure"] {
            design.attributes.insert(Id::new(id), def(id));
        }
        let reserved: Vec<_> = crate::check::check(&design)
            .into_iter()
            .filter(|d| d.code == "def.reserved")
            .collect();
        let ids: Vec<String> = reserved
            .iter()
            .map(|d| match &d.target {
                Some(Target::AttributeDef { id }) => id.clone(),
                other => panic!("対象が属性定義ではありません: {other:?}"),
            })
            .collect();
        assert_eq!(ids, ["Maturity", "count", "label", "part_number"]);
        assert!(reserved.iter().all(|d| d.severity == Severity::Error));
        let message = |i: usize| reserved[i].message.as_str();
        assert!(message(0).contains("部品・カタログ品の"), "{}", message(0));
        assert!(message(1).contains("は部品の"), "{}", message(1));
        assert!(message(3).contains("はカタログ品の"), "{}", message(3));
    }

    /// 組み込みの属性もフィールド名と重ならない（検査はプロジェクト独自の定義だけを見るので、ここで確かめる）。
    #[test]
    fn builtin_attributes_are_not_field_names() {
        for id in builtin::attributes().keys() {
            assert_eq!(field_name_clash(id.as_str()), None, "{id}");
        }
    }

    /// 噴射の状態 `on` の無いスラスタの種別は warning（組み込み・継承で `on` があるものは出ない）。
    #[test]
    fn thruster_kind_without_firing_state_is_warned() {
        let kind = |id: &str, states: &[&str], extends: Option<&str>| {
            let mut k = builtin::kinds()["thruster"].clone();
            k.id = Id::new(id);
            k.states = states.iter().map(|s| s.to_string()).collect();
            k.extends = extends.map(Id::new);
            k.role = extends.is_none().then_some(crate::role::Role::Thruster);
            k
        };
        let mut design = Design::default();
        for k in [
            kind("burner", &["off", "fire"], None),
            kind("burner_ok", &["off", "on"], None),
            kind("burner_child", &[], Some("burner")),
        ] {
            design.kinds.insert(k.id.clone(), k);
        }
        let found: Vec<_> = crate::check::check(&design)
            .into_iter()
            .filter(|d| d.code == "def.thruster_firing_state")
            .collect();
        let ids: Vec<_> = found
            .iter()
            .map(|d| match &d.target {
                Some(Target::KindDef { id }) => id.clone(),
                other => panic!("対象が種別定義ではありません: {other:?}"),
            })
            .collect();
        assert_eq!(ids, ["burner", "burner_child"]);
        assert!(found.iter().all(|d| d.severity == Severity::Warning));
        assert!(found[0].message.contains("`on`"), "{}", found[0].message);
    }
}
