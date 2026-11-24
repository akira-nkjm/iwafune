//! `structure.launch_fit` の射影。打ち上げ時の質量（質量バジェットと同じ部品の行（成長余裕つき）+ 積む推薬）・収納外形（形態が効いていれば全部品の箱、無ければ構体の外形）・固有振動数の見積もり
//! （粗い有限要素 `structure.modal_fem` と簡易固有振動数 `structure.modal_estimate`。どちらも任意の入力で、組み立てられなければ理由を残して使わない）と、
//! 打ち上げ機・ディスペンサー（と要求）の限度（読み出しと優先順は [`crate::launch`]）を解析の入力にする。

use crate::context::Context;
use crate::launch::{
    LaunchValues, LauncherDeparture, capability, frequency_limits, mass_limits, need_selection,
};
use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::design::launch::attr;
use sat_analysis::propulsion::{DepartureProvider, Maneuvers, Propellant};
use sat_analysis::spec::Analysis;
use sat_analysis::structure::interference::InterferenceBox;
use sat_analysis::structure::launch_fit::{
    FEM_MODE_COUNT, LaunchFitDirection, LaunchFitEnvelope, LaunchFitInput, LaunchFitMode,
    LaunchFitPartBoxes, LaunchFitSearch, LaunchFitSource, LaunchFitSourceNote,
};
use sat_analysis::structure::modal_estimate::ModalEstimateKind;
use sat_analysis::structure::modal_fem::ModalFemModeClass;
use sat_analysis::structure::{ModalEstimate, ModalFem};
use sat_astro::math::Vec3;
use sat_units::typed::{Mass, Qty};

/// 固有振動数の見積もりの並びと、出どころごとの「探した範囲」「使えなかった理由」。
#[derive(Default)]
struct FrequencySources {
    modes: Vec<LaunchFitMode>,
    searched: Vec<LaunchFitSearch>,
    unavailable: Vec<LaunchFitSourceNote>,
}

/// 機体全体のモードの向き（機軸方向か横か）。
fn direction_of(class: ModalFemModeClass) -> Option<Option<LaunchFitDirection>> {
    match class {
        ModalFemModeClass::Axial => Some(Some(LaunchFitDirection::Axial)),
        ModalFemModeClass::Lateral => Some(Some(LaunchFitDirection::Lateral)),
        ModalFemModeClass::Local => Some(None),
        ModalFemModeClass::Rigid => None,
    }
}

/// 簡易固有振動数の結果（`structure.modal_estimate`。梁の一次は機体全体、パネルの面外は局所）。見積もれなければ空。
fn beam_modes(ctx: &Context<'_>) -> Vec<LaunchFitMode> {
    let Ok(input) = crate::structure::modal_estimate(ctx) else {
        return Vec::new();
    };
    ModalEstimate::run(&input).map_or_else(
        |_| Vec::new(),
        |out| {
            out.results
                .into_iter()
                .map(|r| LaunchFitMode {
                    id: r.id,
                    source: LaunchFitSource::BeamEstimate,
                    direction: match r.kind {
                        ModalEstimateKind::BeamLateral => Some(LaunchFitDirection::Lateral),
                        ModalEstimateKind::BeamAxial => Some(LaunchFitDirection::Axial),
                        ModalEstimateKind::Panel => None,
                    },
                    frequency: r.frequency,
                })
                .collect()
        },
    )
}

/// 粗い有限要素の結果（`structure.modal_fem`。モードの数は [`FEM_MODE_COUNT`]）。剛体のモードは除く。
/// 組み立てられない・解けないときは、その理由だけを返す（`launch_fit` の不足入力にはしない。有限要素は任意の入力）。
fn finite_element(ctx: &Context<'_>) -> FrequencySources {
    let source = LaunchFitSource::FiniteElement;
    let unavailable = |reason: String| FrequencySources {
        unavailable: vec![LaunchFitSourceNote { source, reason }],
        ..FrequencySources::default()
    };
    let mut input = match crate::structure::modal_fem(ctx) {
        Ok(input) => input,
        Err(missing) => {
            let reasons: Vec<String> = missing.into_vec().into_iter().map(|m| m.reason).collect();
            return unavailable(reasons.join("; "));
        }
    };
    input.mode_count = Some(FEM_MODE_COUNT);
    let out = match ModalFem::run(&input) {
        Ok(out) => out,
        Err(e) => return unavailable(e.to_string()),
    };
    let searched = out
        .modes
        .iter()
        .map(|m| m.frequency)
        .max_by(|a, b| a.value().total_cmp(&b.value()))
        .map(|up_to| LaunchFitSearch { source, up_to });
    FrequencySources {
        modes: out
            .modes
            .iter()
            .filter_map(|m| {
                Some(LaunchFitMode {
                    id: format!("mode {}", m.index),
                    source,
                    direction: direction_of(m.class)?,
                    frequency: m.frequency,
                })
            })
            .collect(),
        searched: searched.into_iter().collect(),
        unavailable: Vec::new(),
    }
}

/// 固有振動数の見積もりの出どころ（梁の見積もり・有限要素）をまとめる。どの出どころが効くかの規則は解析にある。
fn frequency_sources(ctx: &Context<'_>) -> FrequencySources {
    let mut all = finite_element(ctx);
    all.modes.extend(beam_modes(ctx));
    all
}

/// 積む推薬（推進系が無ければ `None`。推薬は `propulsion.propellant` の積む量）。
fn propellant(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Qty<Mass>> {
    if !crate::propulsion::present(ctx) {
        return None;
    }
    let input = match crate::propulsion::maneuvers(ctx) {
        Ok(input) => input,
        Err(e) => {
            m.extend(e);
            return None;
        }
    };
    match Propellant::run(&input) {
        Ok(out) => Some(out.loaded_propellant),
        Err(e) => {
            m.push(
                Target::any_part("propellant"),
                format!("Cannot compute the loaded propellant ({e})"),
            );
            None
        }
    }
}

/// 出発を打ち上げ機が受け持つ最初の遷移の C3（実行の順。無ければ、または求められなければ `None`。
/// 求められないときは不足入力に記録する）。曲線を引く C3 の出どころは推進系の解析 `propulsion.maneuvers` の結果だけ。
fn launcher_departure(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<LauncherDeparture> {
    if !crate::propulsion::present(ctx) {
        return None;
    }
    let input = crate::propulsion::maneuvers(ctx).ok()?;
    let id = input.maneuvers.iter().find_map(|x| {
        x.transfer
            .filter(|t| t.departure_by == DepartureProvider::Launcher)
            .map(|_| x.id.clone())
    })?;
    match Maneuvers::run(&input) {
        Ok(out) => out
            .maneuvers
            .into_iter()
            .find_map(|r| r.dated_transfer.filter(|d| d.maneuver == id))
            .map(|d| LauncherDeparture {
                maneuver: id,
                c3: d.c3,
            }),
        Err(e) => {
            m.push(
                Target::maneuver(id.as_str(), Some("transfer")),
                format!("Cannot compute the departure C3 of maneuver {id} ({e})"),
            );
            None
        }
    }
}

/// 収納外形の入力 `(根の寸法, 部品の箱)`。形態が効いている（打ち上げ条件か実行時の指定が形態を指す）ときは、
/// 寸法を持つ全部品の箱（[`crate::structure::boxes`]。姿勢は形態のもの）を渡し、根の寸法は読まない。効いていなければ根の寸法（従来どおり）。
/// どちらから外形を出すかの規則は解析にある。外形を判定するとき（許容の外形がある）に測るものが無ければ、不足入力にする。
fn stowed_outline(
    ctx: &Context<'_>,
    judged: bool,
    m: &mut MissingInputs,
) -> (Option<Vec3>, Option<LaunchFitPartBoxes>) {
    if ctx.configuration().is_some() {
        let body = crate::structure::boxes(ctx);
        if judged && body.boxes.is_empty() {
            m.push(
                Target::any_part("size"),
                "A part with a size (its box makes the stowed envelope in the configuration)",
            );
        }
        let part_boxes = LaunchFitPartBoxes {
            boxes: body.boxes.into_iter().map(InterferenceBox::from).collect(),
            unsized_parts: body.unsized_parts,
        };
        return (None, Some(part_boxes));
    }
    let size = crate::aocs::reference_box(ctx, m);
    if judged && size.is_none() {
        m.push(
            Target::any_part("size"),
            "Stowed outer dimensions (the `size` of the root part of the mounting tree)",
        );
    }
    (size, None)
}

/// `structure.launch_fit`: 打ち上げ時の質量（乾燥 + 積む推薬）・収納外形（形態が効いていれば全部品の箱、無ければ構体の外形 `aocs::reference_box`）・
/// 簡易固有振動数と、打ち上げ機・ディスペンサー（と要求）の限度。
pub fn launch_fit(ctx: &Context<'_>) -> Result<LaunchFitInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let launch = LaunchValues::new(ctx);
    need_selection(&launch, &mut m);
    let parts = crate::parts::mass_budget_items(ctx, &mut m);
    if parts.is_empty() {
        m.push(Target::any_part("mass"), "No part with a mass");
    }
    let propellant = propellant(ctx, &mut m);
    let envelope = launch.vec3(attr::ENVELOPE).map(|f| LaunchFitEnvelope {
        origin: f.origin,
        size: Vec3::new(f.value[0], f.value[1], f.value[2]),
    });
    let (size, part_boxes) = stowed_outline(ctx, envelope.is_some(), &mut m);
    let (min_lateral, min_axial) = (
        frequency_limits(ctx, &launch, "lateral"),
        frequency_limits(ctx, &launch, "axial"),
    );
    let max_mass = mass_limits(ctx, &launch);
    let capability = launcher_departure(ctx, &mut m).and_then(|d| capability(&launch, &d));
    m.finish(|| {
        let sources = frequency_sources(ctx);
        Some(LaunchFitInput {
            parts,
            propellant_mass: propellant,
            size,
            part_boxes,
            max_mass,
            capability,
            envelope,
            min_lateral,
            min_axial,
            modes: sources.modes,
            searched: sources.searched,
            unavailable: sources.unavailable,
        })
    })
}
