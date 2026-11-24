//! 構造の射影: `structure.mass_properties`（と、姿勢の解析が使う慣性）。慣性の計算は sat-analysis に任せる。
//! 部品の箱の置き場所は子の `body_boxes`、`structure.interference` の射影は子の `interference`、`structure.cone_clearance`（噴流・視野）の射影と担い手の表は子の `cone_clearance`。

mod body_boxes;
mod cone_clearance;
mod interference;

pub use body_boxes::{BodyBox, BodyBoxes, Placed, boxes, placed};
pub use cone_clearance::{CARRIERS, CarrierRow, cone_clearance};
pub use interference::interference;

use crate::context::Context;
use crate::launch::{LaunchValues, frequency_limits, launch_path};
use crate::missing::MissingInputs;
use crate::quantity::{qty_opt, setting, setting_opt};
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::launch::attr;
use fune_model::design::setting_dims::{self as dims, SettingQuantity};
use fune_model::design::{AnalysisSystem, BudgetKind};
use fune_model::resolve::SiValue;
use sat_analysis::structure::launch_fit::tightest_min_frequency;
use sat_analysis::structure::mass_properties::{MassPropertiesItem, MassPropertiesSummary};
use sat_analysis::structure::modal_estimate::{
    ModalEstimateMounted, ModalEstimatePanel, ModalEstimateRequirements,
};
use sat_analysis::structure::static_load::{StaticLoadCase, StaticLoadInterface, StaticLoadItem};
use sat_analysis::structure::{
    MassPropertiesInput, ModalEstimateInput, StaticLoadInput, mass_properties as compute,
};
use sat_units::typed::{Acceleration, Length, Mass, Qty};

/// 質量を持つ部品の質量特性の項目（位置は取付をたどった機体座標の部品原点。外形 `size` があれば箱として慣性を足す）。
pub fn items(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<MassPropertiesItem> {
    let mut out = Vec::new();
    for p in ctx.resolved.parts.values() {
        // 1 個あたりの質量と員数を渡す（員数を掛けるのは解析）。
        let Some(mass) = crate::parts::mass_or_missing(ctx, p, m) else {
            continue;
        };
        let at = placed(ctx, p);
        out.push(MassPropertiesItem {
            id: at.id,
            mass: Qty::from_raw(mass),
            count: p.count_or_one(),
            position: at.position,
            size: at.size,
            orientation: Some(at.orientation),
            inertia: None,
        });
    }
    if out.is_empty() {
        m.push(Target::any_part("mass"), "No part with a mass");
    }
    out
}

/// `structure.mass_properties`
pub fn mass_properties(ctx: &Context<'_>) -> Result<MassPropertiesInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let items = items(ctx, &mut m);
    let limit = ctx.budget_limit(BudgetKind::Mass);
    let target = Target::requirement(
        limit.as_ref().map_or("", |c| c.requirement.as_str()),
        Some("constraint.value"),
    );
    let max_mass = qty_opt::<Mass>(limit.as_ref().map(|c| &c.value), target, &mut m);
    m.finish(|| {
        Some(MassPropertiesInput {
            items,
            max_mass,
            com_box: None,
        })
    })
}

/// 機体の質量特性（姿勢の解析に慣性を渡すため）。計算は `sat_analysis::structure::mass_properties`。
pub fn summary(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<MassPropertiesSummary> {
    let items = items(ctx, m);
    match compute(&items) {
        Ok(s) => Some(s),
        Err(e) => {
            let target = Target::any_part("mass");
            m.push(target, format!("Cannot compute the mass properties ({e})"));
            None
        }
    }
}

/// 荷重倍数（`g0` の倍数）。`conditions/launch.toml` に書いた値が先で、無ければディスペンサー・打ち上げ機の属性 `attr_id`。
/// 書かれた値は単位が無ければそのまま、あれば `g0` に換算する（換算は sat-units）。
fn load_factor(
    q: Option<&fune_model::value::Quantity>,
    key: &SettingQuantity<Acceleration>,
    attr_id: &str,
    values: &LaunchValues<'_>,
    m: &mut MissingInputs,
) -> Option<f64> {
    let g0 = |si: f64| sat_units::parse_unit("g0").ok().map(|g| g.from_si(si));
    let Some(q) = q else {
        let found = values.scalar(attr_id).map(|f| f.value);
        return m
            .need(
                found,
                key.at(&[]),
                format!(
                    "Quasi-static load (g0): write it in conditions/launch.toml or give the `{attr_id}` attribute to the dispenser or launcher"
                ),
            )
            .and_then(g0);
    };
    if q.unit.is_empty() {
        return Some(q.value);
    }
    // 書かれているのに単位が読めないときは、対象を名指しして不足入力にする（汎用の「入力を作れない」にしない）
    match fune_model::value::units::to_si(q.value, &q.unit) {
        Ok(si) => g0(si),
        Err(e) => {
            m.push(key.at(&[]), format!("Quasi-static load (g0): {e}"));
            None
        }
    }
}

/// 準静的荷重の項目（質量を持つ部品。取付の根の取付先は `LV`）。
fn load_items(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<StaticLoadItem> {
    ctx.resolved
        .parts
        .values()
        .filter_map(|p| {
            Some(StaticLoadItem::new(
                p.id.clone(),
                Qty::from_raw(crate::parts::mass_or_missing(ctx, p, m)?),
                p.count_or_one(),
                p.mount.as_ref().map(|mt| mt.on.clone()),
                None,
            ))
        })
        .collect()
}

/// 取付部の許容荷重（締結 1 本の許容せん断荷重を、取付先ごとに横方向の許容として置く）。
fn interfaces(
    ctx: &Context<'_>,
    items: &[StaticLoadItem],
    m: &mut MissingInputs,
) -> Vec<StaticLoadInterface> {
    let allowable = ctx
        .design
        .analysis
        .structure
        .as_ref()
        .and_then(|s| s.fastener_allowable.as_ref());
    let Some(a) = setting_opt(&dims::STRUCTURE_FASTENER_ALLOWABLE, &[], allowable, m) else {
        return Vec::new();
    };
    let mut mounts: Vec<String> = items.iter().map(|i| i.mount.clone()).collect();
    mounts.sort();
    mounts.dedup();
    mounts
        .into_iter()
        .map(|mount| StaticLoadInterface {
            mount,
            allowable_axial: None,
            allowable_lateral: Some(a),
            allowable_moment: None,
        })
        .collect()
}

/// `structure.static_load`: 打ち上げ荷重（`conditions/launch.toml`）で、取付の木をたどった取付部の荷重。
pub fn static_load(ctx: &Context<'_>) -> Result<StaticLoadInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let launch = ctx.design.conditions.launch.as_ref();
    let launch = m.need(
        launch,
        launch_path(&[]),
        "Launch conditions (conditions/launch.toml)",
    );
    let values = LaunchValues::new(ctx);
    let axial = launch.and_then(|l| {
        let key = &dims::LAUNCH_LOAD_AXIAL;
        load_factor(
            l.load_axial.as_ref(),
            key,
            attr::LOAD_AXIAL,
            &values,
            &mut m,
        )
    });
    let lateral = launch.and_then(|l| {
        let key = &dims::LAUNCH_LOAD_LATERAL;
        load_factor(
            l.load_lateral.as_ref(),
            key,
            attr::LOAD_LATERAL,
            &values,
            &mut m,
        )
    });
    let items = load_items(ctx, &mut m);
    let interfaces = interfaces(ctx, &items, &mut m);
    let safety_factor = launch.and_then(|l| l.safety_factor_yield);
    m.finish(|| {
        let label = launch?.label.clone();
        Some(StaticLoadInput {
            items,
            cases: vec![StaticLoadCase::launch(label, axial?, lateral?)],
            interfaces,
            safety_factor,
        })
    })
}

/// パネル 1 枚（材料・外形・板厚。取付けられた部品の質量を非構造質量にする）。
fn panel(
    ctx: &Context<'_>,
    p: &fune_model::resolve::ResolvedPart,
    m: &mut MissingInputs,
) -> Option<ModalEstimatePanel> {
    let Some(SiValue::Enum(name)) = ctx.attr(p, "material") else {
        m.push(Target::attr(&p.id, "material"), "Panel material");
        return None;
    };
    let mp = AnalysisSystem::Structure
        .setting_path()
        .child(&["material", name]);
    let materials = ctx.design.analysis.structure.as_ref().map(|s| &s.material);
    let mat = m.need(
        materials.and_then(|ms| ms.get(name.as_str())),
        mp.clone(),
        "Material properties",
    )?;
    let size = m.need(
        ctx.vec3(p, "size"),
        Target::attr(&p.id, "size"),
        "Panel outline (2 edges)",
    );
    let thickness = crate::parts::part_qty::<Length>(ctx, p, "thickness", "Panel thickness", m);
    let at = [name.as_str()];
    let e = setting(
        &dims::MATERIAL_YOUNGS_MODULUS,
        &at,
        mat.youngs_modulus.as_ref(),
        "Young's modulus",
        m,
    );
    let rho = setting(
        &dims::MATERIAL_DENSITY,
        &at,
        mat.density.as_ref(),
        "Density",
        m,
    );
    let nu = m.need(
        mat.poisson_ratio,
        mp.child(&["poisson_ratio"]),
        "Poisson's ratio",
    );
    let mut sides = size?;
    sides.sort_by(|a, b| b.total_cmp(a));
    let mounted = mounted_on(ctx, &p.id, m);
    Some(ModalEstimatePanel {
        id: p.id.clone(),
        length_a: Qty::from_raw(sides[0]),
        length_b: Qty::from_raw(sides[1]),
        thickness: thickness?,
        youngs_modulus: e?,
        poisson_ratio: nu?,
        density: rho?,
        mounted,
    })
}

/// `structure.modal_estimate`: 材料（`analysis.structure.material`）と外形・板厚を持つパネル（role = panel）。
pub fn modal_estimate(ctx: &Context<'_>) -> Result<ModalEstimateInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let panels: Vec<ModalEstimatePanel> = crate::parts::by_role(ctx, Role::Panel)
        .filter_map(|p| panel(ctx, p, &mut m))
        .collect();
    // 見積もる対象が 1 つも無い設計は、計算の失敗ではなく入力の不足として知らせる
    if crate::parts::by_role(ctx, Role::Panel).next().is_none() {
        m.push(
            Target::any_of_roles(&[Role::Panel], None),
            "No panel part (role = panel)",
        );
    }
    // 最低固有振動数: ディスペンサー・打ち上げ機の値が基本の限度で、要求（`structure.modal_estimate` の `lateral`・`axial`）は
    // それを厳しくする側にだけ働く（効く 1 つの選び方は `structure.launch_fit` の `tightest_min_frequency`）。無ければ判定しない
    let launch = LaunchValues::new(ctx);
    let frequency =
        |key: &str| tightest_min_frequency(&frequency_limits(ctx, &launch, key)).map(|l| l.value);
    let requirements = ModalEstimateRequirements {
        lateral: frequency(sat_checkkey::family::LATERAL),
        axial: frequency(sat_checkkey::family::AXIAL),
        panel: None,
    };
    m.finish(|| {
        Some(ModalEstimateInput {
            panels,
            beams: Vec::new(),
            requirements,
        })
    })
}

/// パネルに載る部品（1 個あたりの質量と員数。合計は解析が求める）。
fn mounted_on(ctx: &Context<'_>, panel: &str, m: &mut MissingInputs) -> Vec<ModalEstimateMounted> {
    ctx.resolved
        .parts
        .values()
        .filter(|c| c.mount.as_ref().is_some_and(|mt| mt.on == panel))
        .filter_map(|c| {
            Some(ModalEstimateMounted {
                id: c.id.clone(),
                mass: Qty::from_raw(crate::parts::mass_or_missing(ctx, c, m)?),
                count: c.count_or_one(),
            })
        })
        .collect()
}
