//! 全体の粗い有限要素の固有値解析（`structure.modal_fem`）。軸に平行なパネルを 4 節点のシェル要素に分け、
//! 集中質量・分布質量・拘束を与えて、低いほうの固有振動数とモードの向きを求める。
//! 概念設計の傾向と桁を見る水準で、詳細設計の FEM の代わりではない（結合部の剛性・積層・実際の断面形状は扱わない）。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `input`・`output` | 入出力の型 |
//! | `mesh` | 軸ごとの共通の格子、節点（辺・T 字で自動で一致）、パネルのつながり |
//! | `shell` | 4 節点のシェル要素（曲げ MITC4・面内 Q4・ドリリングの罰金）と質量 |
//! | `masses`・`lumped` | 質量行列（パネル・分布質量・集中質量の取り付け） |
//! | `supports` | 拘束の種類 → 固定する自由度 |
//! | `assemble` | 剛性行列（スカイライン）の組み立てと自由度の番号 |
//! | `solve` | 固有値（`sat_astro::math::lowest_modes`） |
//! | `classify` | 有効質量の割合とモードの分類 |
//!
//! 前提: 要素は平らで長方形、パネルは節点で剛につなぐ（辺・T 字で格子点が一致するとつながる）、質量は対角の集中質量。
//! 集中質量は取り付け先のパネルの面に投影して配る（面からのオフセットの回転慣性は考えない）。
//! パネルの `mass` を与えると質量行列だけが変わり、剛性は密度と板厚から決まる。
//! 面内回転（ドリリング）の罰金の倍率は [`DRILLING_FACTOR`]（結果にほとんど効かないことをテストで確かめている）。

mod assemble;
mod classify;
mod input;
mod lumped;
mod masses;
mod mesh;
mod output;
mod shell;
mod solve;
mod supports;
mod validate;

#[cfg(test)]
mod tests;

pub use classify::{GLOBAL_MODE_MIN_FRACTION, RIGID_BODY_MAX_HZ};
pub use input::{
    ModalFemAxis, ModalFemFace, ModalFemFixedItem, ModalFemInput, ModalFemLumped, ModalFemPanel,
    ModalFemSkipped, ModalFemSupport, ModalFemSupportError, ModalFemTarget,
};
pub use mesh::{DEFAULT_MERGE_TOLERANCE, DEFAULT_MESH_CAP, DEFAULT_MESH_DIVISIONS};
pub use output::{
    ModalFemAttachment, ModalFemJoint, ModalFemMode, ModalFemModeClass, ModalFemModelSize,
    ModalFemOutput,
};
pub use shell::SHEAR_CORRECTION;

use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, System};
use sat_units::typed::{Fraction, Qty};
use shell::ShellProps;

/// ドリリングの罰金の既定の倍率（G·t に対する比）。面内回転の自由度を面内の回転に結びつける。
pub const DRILLING_FACTOR: f64 = 0.1;
/// 求めるモードの既定の数。
pub const DEFAULT_MODE_COUNT: u32 = 6;

/// 全体の粗い有限要素の固有値解析。
pub struct ModalFem;

impl Analysis for ModalFem {
    const ID: &'static str = "structure.modal_fem";
    const TITLE: &'static str = "粗い有限要素の固有値解析";
    const AREA: AnalysisArea = AnalysisArea::System(System::Structure);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "パネルをシェル要素に分けた機体全体の低次の固有振動数とモードの向き";
    const TITLE_EN: &'static str = "Finite-element modal analysis";
    const DESCRIPTION_EN: &'static str = "Lowest natural frequencies and mode directions of the whole body, with the panels meshed into shell elements";
    type Input = ModalFemInput;
    type Output = ModalFemOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        validate::check(input)?;
        let mesh = mesh::build(input)?;
        let node_masses = masses::build(&mesh, input)?;
        let tol = input
            .merge_tolerance
            .map_or(DEFAULT_MERGE_TOLERANCE, Qty::value);
        let fixed = supports::fixed_dofs(&mesh, &input.support, input.launch_axis, tol)?;
        let props = shell_props(input);
        let sys = assemble::assemble(&mesh, &props, &fixed, &node_masses)?;
        let count = input.mode_count.unwrap_or(DEFAULT_MODE_COUNT) as usize;
        if count == 0 || count > sys.global.len() {
            return Err(AnalysisError::InvalidInput(format!(
                "The mode count must be 1 to {} for this model",
                sys.global.len()
            )));
        }
        let solved = solve::solve(&sys, count)?;
        let total = node_masses.total();
        let data = classify::classify(&sys, &solved, total, input.launch_axis);
        Ok(output(
            &mesh,
            &fixed,
            &node_masses,
            &data,
            &solved,
            &input.skipped_panels,
        ))
    }
}

/// パネルごとの要素の材料。
fn shell_props(input: &ModalFemInput) -> Vec<ShellProps> {
    input
        .panels
        .iter()
        .map(|p| ShellProps {
            youngs_modulus: p.youngs_modulus.value(),
            poisson_ratio: p.poisson_ratio,
            thickness: p.thickness.value(),
            drilling_factor: input.drilling_factor.unwrap_or(DRILLING_FACTOR),
        })
        .collect()
}

/// 分類の結果を出力のモードの並びにする。
fn modes_of(data: &[classify::ModeData]) -> Vec<ModalFemMode> {
    data.iter()
        .enumerate()
        .map(|(i, m)| ModalFemMode {
            index: i as u32 + 1,
            frequency: Qty::from_raw(m.frequency_hz),
            effective_mass_fraction: m.fractions.map(Fraction::new),
            class: m.class,
            dominant_axis: m.dominant,
        })
        .collect()
}

fn output(
    mesh: &mesh::Mesh,
    fixed: &[bool],
    node_masses: &masses::NodeMasses,
    data: &[classify::ModeData],
    solved: &solve::Solved,
    skipped: &[ModalFemSkipped],
) -> ModalFemOutput {
    let first = |class: ModalFemModeClass| {
        data.iter()
            .find(|m| m.class == class)
            .map(|m| Qty::from_raw(m.frequency_hz))
    };
    ModalFemOutput {
        modes: modes_of(data),
        rigid_body_modes: data
            .iter()
            .filter(|m| m.class == ModalFemModeClass::Rigid)
            .count() as u32,
        lowest_lateral: first(ModalFemModeClass::Lateral),
        lowest_axial: first(ModalFemModeClass::Axial),
        model: ModalFemModelSize {
            nodes: mesh.node_count() as u32,
            elements: mesh.element_count() as u32,
            dofs: fixed.len() as u32,
            fixed_dofs: fixed.iter().filter(|f| **f).count() as u32,
            mesh_size: Qty::from_raw(mesh.mesh_size),
        },
        joints: mesh
            .joints
            .iter()
            .map(|j| ModalFemJoint {
                panel_a: mesh.panels[j.a].id.clone(),
                panel_b: mesh.panels[j.b].id.clone(),
                shared_nodes: j.shared_nodes as u32,
            })
            .collect(),
        attachments: node_masses
            .attachments
            .iter()
            .map(|a| ModalFemAttachment {
                id: a.id.clone(),
                panel: a.panel.clone(),
                distance: Qty::from_raw(a.distance),
                automatic: a.automatic,
                share: Fraction::new(a.share),
            })
            .collect(),
        skipped_panels: skipped.to_vec(),
        iterations: solved.iterations as u32,
        cluster_split: solved.cluster_split,
        total_mass: Qty::from_raw(node_masses.total()),
    }
}
