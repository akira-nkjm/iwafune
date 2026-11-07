//! 構造の射影: `structure.mass_properties`（と、姿勢の解析が使う慣性）。慣性の計算は sat-analysis に任せる。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::pose;
use crate::quantity::{qty, qty_opt};
use fune_model::resolve::SiValue;
use sat_analysis::structure::mass_properties::{MassPropertiesItem, MassPropertiesSummary};
use sat_analysis::structure::modal_estimate::{
    ModalEstimateMounted, ModalEstimatePanel, ModalEstimateRequirements,
};
use sat_analysis::structure::static_load::{StaticLoadCase, StaticLoadInterface, StaticLoadItem};
use sat_analysis::structure::{
    MassPropertiesInput, ModalEstimateInput, StaticLoadInput, mass_properties as compute,
};
use sat_astro::math::Vec3;
use sat_units::typed::{Force, Frequency, Length, Mass, MassDensity, Pressure, Qty};

/// 質量を持つ部品の質量特性の項目（位置は取付をたどった機体座標の部品原点。外形 `size` があれば箱として慣性を足す）。
pub fn items(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<MassPropertiesItem> {
    let mut out = Vec::new();
    for p in ctx.resolved.parts.values() {
        // 1 個あたりの質量と員数を渡す（員数を掛けるのは解析）。
        let Some(mass) = p.scalar("mass") else {
            if p.attr.contains_key("mass") {
                m.push(format!("{}.mass", p.id), "部品の質量");
            }
            continue;
        };
        let (position, orientation) = pose::body_pose(ctx.resolved, &p.id)
            .unwrap_or((Vec3::ZERO, sat_astro::math::Quat::IDENTITY));
        out.push(MassPropertiesItem {
            id: p.id.clone(),
            mass: Qty::from_raw(mass),
            count: crate::parts::count_of(p),
            position,
            size: p.vec3("size").map(|s| Vec3::new(s[0], s[1], s[2])),
            orientation: Some(orientation),
            inertia: None,
        });
    }
    if out.is_empty() {
        m.push("<部品>.mass", "質量を持つ部品がありません");
    }
    out
}

/// `structure.mass_properties`
pub fn mass_properties(ctx: &Context<'_>) -> Result<MassPropertiesInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let items = items(ctx, &mut m);
    let limit = ctx.design.budget.allocation.mass.as_ref().map(|a| &a.limit);
    let max_mass = qty_opt::<Mass>(limit, "budget.mass.limit", &mut m);
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
            m.push("<部品>.mass", format!("質量特性を求められません（{e}）"));
            None
        }
    }
}

/// 荷重倍数（`g0` の倍数）。単位が無ければそのまま、あれば `g0` に換算する（換算は sat-units）。
fn load_factor(
    q: Option<&fune_model::value::Quantity>,
    path: &str,
    m: &mut MissingInputs,
) -> Option<f64> {
    let q = m.need(q, path, "準静的荷重（g0）")?;
    if q.unit.is_empty() {
        return Some(q.value);
    }
    let si = fune_model::value::units::to_si(q.value, &q.unit).ok()?;
    sat_units::parse_unit("g0").ok().map(|g| g.from_si(si))
}

/// 準静的荷重の項目（質量を持つ部品。取付の根の取付先は `LV`）。
fn load_items(ctx: &Context<'_>) -> Vec<StaticLoadItem> {
    ctx.resolved
        .parts
        .values()
        .filter_map(|p| {
            Some(StaticLoadItem {
                id: p.id.clone(),
                mass: Qty::from_raw(p.scalar("mass")?),
                count: crate::parts::count_of(p),
                mount: p
                    .mount
                    .as_ref()
                    .map_or_else(|| "LV".to_owned(), |mt| mt.on.clone()),
                cg_height: None,
            })
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
    let Some(a) = qty_opt::<Force>(allowable, "analysis.structure.fastener_allowable", m) else {
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

/// `structure.static_load`: 打ち上げ荷重（`mission/launch.toml`）で、取付の木をたどった取付部の荷重。
pub fn static_load(ctx: &Context<'_>) -> Result<StaticLoadInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let launch = ctx.design.mission.launch.as_ref();
    let launch = m.need(
        launch,
        "mission.launch",
        "打ち上げ条件（mission/launch.toml）",
    );
    let axial = launch
        .and_then(|l| load_factor(l.load_axial.as_ref(), "mission.launch.load_axial", &mut m));
    let lateral = launch.and_then(|l| {
        load_factor(
            l.load_lateral.as_ref(),
            "mission.launch.load_lateral",
            &mut m,
        )
    });
    let items = load_items(ctx);
    let interfaces = interfaces(ctx, &items, &mut m);
    let safety_factor = launch.and_then(|l| l.safety_factor_yield);
    m.finish(|| {
        let label = launch?
            .label
            .clone()
            .unwrap_or_else(|| "打ち上げ".to_owned());
        Some(StaticLoadInput {
            items,
            cases: vec![StaticLoadCase {
                id: "launch".to_owned(),
                label,
                axial: axial?,
                lateral: lateral?,
            }],
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
    let Some(SiValue::Enum(name)) = p.value("material") else {
        m.push(format!("{}.material", p.id), "パネルの材料");
        return None;
    };
    let mp = format!("analysis.structure.material.{name}");
    let materials = ctx.design.analysis.structure.as_ref().map(|s| &s.material);
    let mat = m.need(
        materials.and_then(|ms| ms.get(name.as_str())),
        mp.as_str(),
        "材料の物性",
    )?;
    let size = m.need(
        p.vec3("size"),
        format!("{}.size", p.id),
        "パネルの外形（2 辺）",
    );
    let thickness = crate::parts::part_qty::<Length>(p, "thickness", "パネルの板厚", m);
    let e = qty::<Pressure>(
        mat.youngs_modulus.as_ref(),
        &format!("{mp}.youngs_modulus"),
        "ヤング率",
        m,
    );
    let rho = qty::<MassDensity>(mat.density.as_ref(), &format!("{mp}.density"), "密度", m);
    let nu = m.need(
        mat.poisson_ratio,
        format!("{mp}.poisson_ratio"),
        "ポアソン比",
    );
    let mut sides = size?;
    sides.sort_by(|a, b| b.total_cmp(a));
    let mounted = mounted_on(ctx, &p.id);
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
    let panels: Vec<ModalEstimatePanel> = crate::parts::by_role(ctx, "panel")
        .filter_map(|p| panel(ctx, p, &mut m))
        .collect();
    let launch = ctx.design.mission.launch.as_ref();
    let freq = |q: Option<&fune_model::value::Quantity>, path: &str, m: &mut MissingInputs| {
        qty_opt::<Frequency>(q, path, m)
    };
    let requirements = ModalEstimateRequirements {
        lateral: freq(
            launch.and_then(|l| l.min_frequency_lateral.as_ref()),
            "mission.launch.min_frequency_lateral",
            &mut m,
        ),
        axial: freq(
            launch.and_then(|l| l.min_frequency_axial.as_ref()),
            "mission.launch.min_frequency_axial",
            &mut m,
        ),
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
fn mounted_on(ctx: &Context<'_>, panel: &str) -> Vec<ModalEstimateMounted> {
    ctx.resolved
        .parts
        .values()
        .filter(|c| c.mount.as_ref().is_some_and(|mt| mt.on == panel))
        .filter_map(|c| {
            Some(ModalEstimateMounted {
                id: c.id.clone(),
                mass: Qty::from_raw(c.scalar("mass")?),
                count: crate::parts::count_of(c),
            })
        })
        .collect()
}
