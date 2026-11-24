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
        ValueType::Scalar
            | ValueType::Vec3
            | ValueType::Range
            | ValueType::Tensor3
            | ValueType::Curve
    );
    if !needs_unit && (def.dimension.is_some() || !def.units.is_empty()) {
        out.push((
            "def.dimension",
            format!(
                "An attribute of type {:?} cannot have a dimension or units",
                def.value_type
            ),
        ));
    }
    match (&def.at_dimension, def.value_type) {
        (Some(dim), ValueType::Curve) if parse_dimension_name(dim).is_err() => {
            out.push(("def.dimension", format!("Unknown dimension name `{dim}`")));
        }
        (None, ValueType::Curve) => out.push((
            "def.dimension",
            "A curve needs the dimension of its argument (at_dimension)".to_string(),
        )),
        (Some(_), t) if t != ValueType::Curve => out.push((
            "def.dimension",
            "Only a curve can have an argument dimension (at_dimension)".to_string(),
        )),
        _ => {}
    }
    if def.by_state && def.value_type != ValueType::Scalar {
        out.push((
            "def.by_state",
            "Per-state values are only allowed for scalars".to_string(),
        ));
    }
    if def.fov_axis.is_some() && def.value_type != ValueType::Scalar {
        out.push((
            "def.fov_axis",
            "A field-of-view axis can only be given to a scalar (the half angle)".to_string(),
        ));
    }
    match &def.dimension {
        Some(dim) => {
            if parse_dimension_name(dim).is_err() {
                out.push(("def.dimension", format!("Unknown dimension name `{dim}`")));
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
                        format!(
                            "The dimension of allowed unit `{unit}` does not match the other units"
                        ),
                    )),
                    _ => {}
                }
            }
            if needs_unit && def.units.is_empty() {
                out.push((
                    "def.dimension",
                    "Specify a dimension (dimension) or allowed units (units)".to_string(),
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
        (PartFieldName::NAMES, "part"),
        (CatalogFieldName::NAMES, "catalog item"),
    ]
    .into_iter()
    .filter(|(fields, _)| clash(fields.iter().copied(), id).is_some())
    .map(|(_, owner)| owner)
    .collect();
    (!owners.is_empty()).then(|| {
        format!(
            "Attribute ID `{id}` is the same as a field name of {} (path notation cannot tell the attribute from the field)",
            owners.join(" and ")
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
                format!(
                    "Attribute `{}` has the same ID as a built-in definition",
                    def.id
                ),
            );
        }
        for (code, message) in attribute_def_problems(def) {
            ctx.error(code, &entity, &[], target(), message);
        }
        if let Some(axis) = &def.fov_axis
            && !design
                .attribute(axis.as_str())
                .is_some_and(|a| a.unit_vector)
        {
            ctx.error(
                "ref.attribute",
                &entity,
                &["fov_axis"],
                target(),
                format!(
                    "Field-of-view axis `{axis}` is not a defined direction (unit vector) attribute"
                ),
            );
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
            format!(
                "Kind `{}` has the same ID as a built-in definition",
                kind.id
            ),
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
            format!("Parent kind `{parent}` is not defined"),
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
            "Kind inheritance is circular",
        );
    }
    for attr in kind.required.iter().chain(&kind.optional) {
        if design.attribute(attr.as_str()).is_none() {
            ctx.error(
                "ref.attribute",
                &entity,
                &[],
                path.clone(),
                format!("Attribute `{attr}` has no definition"),
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
                "Thruster kind `{id}` has no `{}` state. Power during a burn and the thrust-error disturbance are read from state `{}`, so both would be 0",
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

    /// 視野の軸は、定義済みの方向（単位ベクトル）の属性でなければ error。スカラー以外に付けても error。
    #[test]
    fn fov_axis_must_point_at_a_direction_attribute() {
        let codes = |design: &Design| -> Vec<String> {
            crate::check::check(design)
                .into_iter()
                .filter(|d| d.code.starts_with("ref.attribute") || d.code == "def.fov_axis")
                .map(|d| d.code.to_string())
                .collect()
        };
        let mut design = Design::default();
        let mut ok = def("half_view");
        ok.fov_axis = Some(Id::new("boresight"));
        design.attributes.insert(Id::new("half_view"), ok);
        assert!(codes(&design).is_empty());
        let mut bad = def("half_view2");
        bad.fov_axis = Some(Id::new("mass"));
        design.attributes.insert(Id::new("half_view2"), bad);
        assert_eq!(codes(&design), ["ref.attribute"]);
        let mut vec = def("half_view3");
        vec.value_type = ValueType::Vec3;
        vec.fov_axis = Some(Id::new("boresight"));
        assert_eq!(attribute_def_problems(&vec)[0].0, "def.fov_axis");
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
                other => panic!("target is not an attribute definition: {other:?}"),
            })
            .collect();
        assert_eq!(ids, ["Maturity", "count", "label", "part_number"]);
        assert!(reserved.iter().all(|d| d.severity == Severity::Error));
        let message = |i: usize| reserved[i].message.as_str();
        assert!(
            message(0).contains("part and catalog item"),
            "{}",
            message(0)
        );
        assert!(message(1).contains("of part"), "{}", message(1));
        assert!(message(3).contains("of catalog item"), "{}", message(3));
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
                other => panic!("target is not a kind definition: {other:?}"),
            })
            .collect();
        assert_eq!(ids, ["burner", "burner_child"]);
        assert!(found.iter().all(|d| d.severity == Severity::Warning));
        assert!(found[0].message.contains("`on`"), "{}", found[0].message);
    }
}
