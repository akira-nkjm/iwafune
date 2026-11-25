//! スラスタが推薬を取るタンク（部品の属性 `tank`。[`fune_model::design::refs::TANK_ATTR`]）の読み取りと検査。
//!
//! - 読み取り: 役割 `propulsion_thruster` と `thruster`（RCS）の部品が `tank` に書いたタンクの部品の ID を、推進系の入力
//!   （`PropulsionInput::thruster_tanks`）に写す。書かなければ表に載せない（同じ推薬のタンクから取る）。
//!   指す部品が無い・役割が `propellant_tank` でなければ、その属性の不足の入力にする（直し方の候補はタンク）。
//! - 検査（[`check`]）: タンクの推薬とスラスタの推薬の名前の食い違い（`propulsion.tank_propellant`）。部品が無い・役割が違う誤りは
//!   `fune_model::check`（`ref.part`・`propulsion.tank_role`）が、どのスラスタも取らないタンクは `propulsion.propellant`
//!   （`tank_unused`）が見る。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::by_role;
use fune_model::builtin::Role;
use fune_model::design::EntityKey;
use fune_model::design::refs::TANK_ATTR;
use fune_model::resolve::{Resolved, ResolvedPart, SiValue};
use fune_model::{Design, Diagnostic, Target};
use std::collections::BTreeMap;

/// 推薬を取るタンクを書ける部品（推進系のスラスタと RCS のスラスタ）。
fn thrusters<'a>(ctx: &'a Context<'_>) -> impl Iterator<Item = &'a ResolvedPart> {
    by_role(ctx, Role::PropulsionThruster).chain(by_role(ctx, Role::Thruster))
}

/// 部品が書いたタンクの部品の ID（書かれていなければ `None`）。
fn named(ctx: &Context<'_>, p: &ResolvedPart) -> Option<String> {
    match ctx.attr(p, TANK_ATTR)? {
        SiValue::Text(s) | SiValue::Enum(s) => Some(s.clone()),
        _ => None,
    }
}

/// RCS（役割 `thruster`）の部品がタンクを名指しているか（名指していれば、推薬の解析にアンロードの入力が要る）。
pub(super) fn rcs_names_tank(ctx: &Context<'_>) -> bool {
    by_role(ctx, Role::Thruster).any(|p| named(ctx, p).is_some())
}

/// スラスタの部品の ID → 推薬を取るタンクの部品の ID。タンクの部品が無い・役割が違うものは不足の入力にして表に載せない。
pub(super) fn thruster_tanks(ctx: &Context<'_>, m: &mut MissingInputs) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for p in thrusters(ctx) {
        let Some(tank) = named(ctx, p) else {
            continue;
        };
        let is_tank = ctx
            .resolved
            .parts
            .get(tank.as_str())
            .is_some_and(|t| t.role == Some(Role::PropellantTank));
        if is_tank {
            out.insert(p.id.clone(), tank);
        } else {
            m.push(
                Target::attr(&p.id, TANK_ATTR),
                format!(
                    "Part {tank}, the tank of thruster {}, is not a propellant tank",
                    p.id
                ),
            );
        }
    }
    out
}

/// 推薬の名前（`propellant` の列挙・文字列）。
fn propellant(p: &ResolvedPart) -> Option<&str> {
    match p.value("propellant")? {
        SiValue::Enum(s) | SiValue::Text(s) => Some(s.as_str()),
        _ => None,
    }
}

/// タンクの推薬の名前とスラスタの推薬の名前の食い違いの警告（どちらかに名前が無ければ言わない）。
pub fn check(design: &Design, resolved: &Resolved) -> Vec<Diagnostic> {
    let Ok(ctx) = Context::new(design, resolved, None) else {
        return Vec::new();
    };
    let named: Vec<(&ResolvedPart, String)> = thrusters(&ctx)
        .filter_map(|p| named(&ctx, p).map(|t| (p, t)))
        .collect();
    let mut out = Vec::new();
    for (p, tank) in &named {
        let Some(t) = resolved.parts.get(tank.as_str()) else {
            continue;
        };
        if let (Some(a), Some(b)) = (propellant(p), propellant(t))
            && a != b
        {
            out.push(
                Diagnostic::warning(
                    "propulsion.tank_propellant",
                    format!(
                        "Thruster `{}` uses {a}, but its tank `{tank}` holds {b}",
                        p.id
                    ),
                )
                .with_target(Target::attr(&p.id, TANK_ATTR))
                .at(design
                    .source_map
                    .locate(&EntityKey::Part(p.id.clone()), &["attr", TANK_ATTR])),
            );
        }
    }
    out
}
