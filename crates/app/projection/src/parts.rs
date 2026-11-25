//! 解決済みの部品から値を取り出す部品（役割での選択・型付きの SI 量・機体座標の方向）。足りなければパスで記録する。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::pose;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::UncertaintySide;
use fune_model::resolve::{ResolvedPart, SiScalar, SiValue};
use sat_analysis::budget::BudgetItem;
use sat_analysis::modes::StateValues;
use sat_astro::math::Vec3;
use sat_units::typed::{Dimension, Qty};
use std::collections::BTreeMap;

/// 視線（`boresight`）と視野を持つセンサの役割。禁止角（`aocs.sensor_fov`）と視野の遮り（`structure.cone_clearance`）が共有する。
pub const SENSOR_ROLES: &[Role] = &[Role::StarTracker, Role::SunSensor, Role::Camera];

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

    /// 選んだケースの `side`（ケースが無い・書かれていなければ None = nominal）。
    pub fn side(&self) -> Option<UncertaintySide> {
        self.case.and_then(|c| c.side)
    }

    /// 値のスカラーを、ケースの `side` が指す側で読む（`min` / `max` があればそれ、無ければ nominal）。
    /// **部品の属性のスカラー・状態ごとの値を解析の入力にするときは、必ずここを通る**（選択そのものは `SiScalar::read`）。
    pub fn read(&self, s: &SiScalar) -> f64 {
        s.read(self.side())
    }

    /// 部品の属性のスカラー（SI。ケースの `side` が指す側。範囲 `Range` は不確かさではないので対象外）。
    pub fn scalar(&self, p: &ResolvedPart, attr: &str) -> Option<f64> {
        match self.attr(p, attr)? {
            SiValue::Scalar(s) => Some(self.read(s)),
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

/// 系（`ResolvedPart.system`。置き場所の最上位の系 → 種別の役割の系）が `system` の部品（ID 順）。
pub fn in_system<'a>(
    ctx: &'a Context<'_>,
    system: fune_model::design::AnalysisSystem,
) -> impl Iterator<Item = &'a ResolvedPart> + 'a {
    ctx.resolved
        .parts
        .values()
        .filter(move |p| p.system == Some(system))
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
                .map(|(s, v)| (s.clone(), Qty::from_raw(ctx.read(v))))
                .collect(),
        }),
        SiValue::Scalar(s) => Some(StateValues {
            default: Some(Qty::from_raw(ctx.read(s))),
            by_state: BTreeMap::new(),
        }),
        _ => None,
    }
}

/// 表示名（無ければ ID）。
pub fn label(p: &ResolvedPart) -> String {
    fune_model::display_name::name_or_id(p.label.as_deref(), &p.id)
}

/// 員数を掛けた質量 [kg]。
pub fn total_mass(ctx: &Context<'_>, p: &ResolvedPart) -> Option<f64> {
    ctx.scalar(p, "mass").map(|m| extensive(p, m))
}

/// 1 個あたりの質量 [kg]。`mass` を書いてあるのに値が読めない部品は不足入力に記録する
/// （`mass` をまったく書かない部品は質量を持たない部品として、記録せずに `None`）。質量特性・荷重・固有振動の項目で共有する。
pub fn mass_or_missing(ctx: &Context<'_>, p: &ResolvedPart, m: &mut MissingInputs) -> Option<f64> {
    let mass = ctx.scalar(p, "mass");
    if mass.is_none() && p.attr.contains_key("mass") {
        m.push(Target::attr(&p.id, "mass"), "Part mass");
    }
    mass
}

/// count を持たない解析入力に渡す示量の合計値。count があれば単体値を渡す。
pub fn extensive(p: &ResolvedPart, value: f64) -> f64 {
    value * f64::from(p.count_or_one())
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

/// 質量バジェットの 1 行（1 個あたりの値・員数・成熟度から引いた成長余裕）。**行の組み立てはここだけ**
/// （service のバジェットと `structure.launch_fit` の入力が同じ行を使う）。
pub fn budget_item(design: &fune_model::Design, p: &ResolvedPart, value: f64) -> BudgetItem {
    BudgetItem {
        id: p.id.clone(),
        group: String::new(),
        value,
        count: p.count_or_one(),
        growth: growth(design, p).unwrap_or_default(),
    }
}

/// 質量バジェットの部品の行（質量はケースの `side` が指す側。質量の無い部品は数えず、
/// 質量の属性があるのに値が決まらない部品は `m` に記録する）。**質量バジェットの行の集まりはここだけ**。
pub fn mass_budget_items(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<BudgetItem> {
    ctx.resolved
        .parts
        .values()
        .filter_map(|p| Some(budget_item(ctx.design, p, mass_or_missing(ctx, p, m)?)))
        .collect()
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
