//! 解決済みの部品から値を取り出す部品（役割での選択・型付きの SI 量・機体座標の方向）。足りなければパスで記録する。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::pose;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::resolve::{ResolvedPart, SiValue};
use sat_analysis::modes::StateValues;
use sat_astro::math::Vec3;
use sat_units::typed::{Dimension, Qty};
use std::collections::BTreeMap;

/// 部品の属性の読み出し。**射影は部品の属性をここからだけ読む**（読んだ属性を `Context::note` で記録するため）。
impl Context<'_> {
    /// 部品の属性の値（設定されていれば）。部品の種別がその属性を持てば、値が無くても読んだものとして記録する
    /// （書かれていなければ解析の既定値を使う、が分かるように）。
    pub fn attr<'p>(&self, p: &'p ResolvedPart, attr: &str) -> Option<&'p SiValue> {
        if p.attr.contains_key(attr) {
            self.note(Target::attr(&p.id, attr));
        }
        p.value(attr)
    }

    /// 部品の属性のスカラーの nominal（SI）。
    pub fn scalar(&self, p: &ResolvedPart, attr: &str) -> Option<f64> {
        match self.attr(p, attr)? {
            SiValue::Scalar(s) => Some(s.value),
            _ => None,
        }
    }

    /// 部品の属性のベクトル（SI）。
    pub fn vec3(&self, p: &ResolvedPart, attr: &str) -> Option<[f64; 3]> {
        match self.attr(p, attr)? {
            SiValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }
}

/// 役割（kind の role）が `role` の部品（ID 順）。
pub fn by_role<'a>(
    ctx: &'a Context<'_>,
    role: Role,
) -> impl Iterator<Item = &'a ResolvedPart> + 'a {
    ctx.resolved
        .parts
        .values()
        .filter(move |p| p.role == Some(role))
}

/// 属性が設定されている部品（ID 順）。
pub fn with_attr<'a>(
    ctx: &'a Context<'_>,
    attr: &'a str,
) -> impl Iterator<Item = &'a ResolvedPart> + 'a {
    ctx.resolved
        .parts
        .values()
        .filter(move |p| p.value(attr).is_some())
}

/// スカラーの nominal を `Qty<D>` で。無ければ `PART.attr` を記録。
pub fn part_qty<D: Dimension>(
    ctx: &Context<'_>,
    p: &ResolvedPart,
    attr: &str,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Qty<D>> {
    let v = ctx.scalar(p, attr);
    m.need(v, Target::attr(&p.id, attr), reason)
        .map(Qty::from_raw)
}

/// 無次元の比（必須）。無ければ `PART.attr` を記録。省略できる比は `ctx.scalar(p, attr)` をそのまま解析に渡す
/// （既定値は解析が持つ）。
pub fn ratio(
    ctx: &Context<'_>,
    p: &ResolvedPart,
    attr: &str,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<f64> {
    m.need(ctx.scalar(p, attr), Target::attr(&p.id, attr), reason)
}

/// 部品座標で書かれた方向ベクトルを機体座標に直したもの。無ければ記録。
pub fn direction_body(
    ctx: &Context<'_>,
    p: &ResolvedPart,
    attr: &str,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Vec3> {
    let v = m.need(ctx.vec3(p, attr), Target::attr(&p.id, attr), reason)?;
    Some(pose::to_body(
        ctx.resolved,
        &p.id,
        Vec3::new(v[0], v[1], v[2]),
    ))
}

/// 1 台の状態ごとの値の表（`sat_analysis::modes::StateValues`。員数は掛けず、解析に `count` で渡す）。
/// 状態ごとの値はそのまま `by_state`、スカラーは `default`（どの状態でもその値）。値が無いか、スカラーでも
/// 状態ごとでもなければ `None`。**状態ごとの値の写し替えはこれだけ**（電源・C&DH・熱）。
pub fn state_values<D: Dimension>(
    ctx: &Context<'_>,
    p: &ResolvedPart,
    attr: &str,
) -> Option<StateValues<D>> {
    match ctx.attr(p, attr)? {
        SiValue::ByState(map) => Some(StateValues {
            default: None,
            by_state: map
                .iter()
                .map(|(s, v)| (s.clone(), Qty::from_raw(v.value)))
                .collect(),
        }),
        SiValue::Scalar(s) => Some(StateValues {
            default: Some(Qty::from_raw(s.value)),
            by_state: BTreeMap::new(),
        }),
        _ => None,
    }
}

/// 表示名（無ければ ID）。
pub fn label(p: &ResolvedPart) -> String {
    p.label.clone().unwrap_or_else(|| p.id.clone())
}

/// 員数を掛けた質量 [kg]。
pub fn total_mass(ctx: &Context<'_>, p: &ResolvedPart) -> Option<f64> {
    ctx.scalar(p, "mass").map(|m| extensive(p, m))
}

/// 設計の員数（0 は未指定と同じく 1）。解析入力の count と示量の合計で共有する。
pub fn count_of(p: &ResolvedPart) -> u32 {
    p.count.max(1)
}

/// count を持たない解析入力に渡す示量の合計値。count があれば単体値を渡す。
pub fn extensive(p: &ResolvedPart, value: f64) -> f64 {
    value * f64::from(count_of(p))
}

/// 員数を掛けた示量の値を `Qty<D>` で（無ければ `PART.attr` を記録）。
pub fn extensive_qty<D: Dimension>(
    ctx: &Context<'_>,
    p: &ResolvedPart,
    attr: &str,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Qty<D>> {
    part_qty::<D>(ctx, p, attr, reason, m).map(|q| Qty::from_raw(extensive(p, q.value())))
}

/// 成熟度ごとの成長余裕（`budget/policy.toml` の `[growth]`）。**成熟度 → 成長余裕の引き当てはここだけ**（service のバジェットも使う）。
pub fn growth(design: &fune_model::Design, p: &ResolvedPart) -> Option<f64> {
    p.maturity
        .and_then(|mat| design.budget.policy.growth.get(mat))
}

/// 設計の冗長の方式を解析の列挙へ写す。
pub fn redundancy(p: &fune_model::resolve::ResolvedPart) -> sat_analysis::modes::Redundancy {
    use fune_model::design::Redundancy as D;
    use sat_analysis::modes::Redundancy as A;
    match p.redundancy {
        D::None => A::None,
        D::Cold => A::Cold,
        D::Warm => A::Warm,
    }
}
