//! `structure.launch_fit` の射影。打ち上げ時の質量（乾燥 + 積む推薬）・収納外形（構体の外形）・簡易固有振動数と、
//! 打ち上げ機・ディスペンサー（と要求）の限度（読み出しと優先順は [`crate::launch`]）を解析の入力にする。

use crate::context::Context;
use crate::launch::{LaunchValues, frequency_limits, mass_limits, need_selection};
use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::design::launch::attr;
use sat_analysis::propulsion::Propellant;
use sat_analysis::spec::Analysis;
use sat_analysis::structure::ModalEstimate;
use sat_analysis::structure::launch_fit::{LaunchFitEnvelope, LaunchFitInput, LaunchFitMode};
use sat_astro::math::Vec3;
use sat_units::typed::{Mass, Qty};

/// 簡易固有振動数の結果（`structure.modal_estimate`。梁の一次だけが機体全体の条件に使われる）。見積もれなければ空。
fn modes(ctx: &Context<'_>) -> Vec<LaunchFitMode> {
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
                    kind: r.kind,
                    frequency: r.frequency,
                })
                .collect()
        },
    )
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

/// `structure.launch_fit`: 打ち上げ時の質量（乾燥 + 積む推薬）・収納外形（構体の外形。`aocs::reference_box`）・
/// 簡易固有振動数と、打ち上げ機・ディスペンサー（と要求）の限度。
pub fn launch_fit(ctx: &Context<'_>) -> Result<LaunchFitInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let launch = LaunchValues::new(ctx);
    need_selection(&launch, &mut m);
    let dry = crate::structure::summary(ctx, &mut m);
    let propellant = propellant(ctx, &mut m);
    let envelope = launch.vec3(attr::ENVELOPE).map(|f| LaunchFitEnvelope {
        origin: f.origin,
        size: Vec3::new(f.value[0], f.value[1], f.value[2]),
    });
    let size = crate::aocs::reference_box(ctx, &mut m);
    if envelope.is_some() && size.is_none() {
        m.push(
            Target::any_part("size"),
            "Stowed outer dimensions (the `size` of the root part of the mounting tree)",
        );
    }
    let (min_lateral, min_axial) = (
        frequency_limits(ctx, &launch, "lateral"),
        frequency_limits(ctx, &launch, "axial"),
    );
    let max_mass = mass_limits(ctx, &launch);
    m.finish(|| {
        Some(LaunchFitInput {
            dry_mass: dry?.total_mass,
            propellant_mass: propellant,
            size,
            max_mass,
            envelope,
            min_lateral,
            min_axial,
            modes: modes(ctx),
        })
    })
}
