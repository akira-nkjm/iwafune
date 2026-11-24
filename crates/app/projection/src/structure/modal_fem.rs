//! `structure.modal_fem` の射影。パネル（role = panel。読み方は [`super::panel`]）を、機体座標の中央面の長方形にし、
//! それ以外の質量を持つ部品を、重心の位置の集中質量にする。構体のフレーム（role = structure）は質量だけを全体に配る。
//! 物理のモデル化の規則（つなぎ方・拘束の意味・既定値）は解析が持つ。ここは設計データの読み出しと、
//! 「どのパネルにつなぐか」の決め方（設定 `[modal].tie` → 取付先のパネル → 解析が選ぶ最も近いパネル）だけ。
//!
//! 打ち上げの解析なので、部品は打ち上げの形態の姿勢で置かれる（展開物は収納の姿勢のまま、ほかの部品と同じ集中質量になる）。
//! 質量は成長余裕を含まない値（質量バジェットの素の質量）。

use super::body_boxes::placed;
use super::panel;
use crate::context::Context;
use crate::launch::{LaunchValues, TextSet, launch_path};
use crate::missing::MissingInputs;
use crate::outline::axis_aligned_extents;
use crate::parts::{extensive, mass_or_missing};
use crate::quantity::setting_opt;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::AnalysisSystem;
use fune_model::design::launch::attr;
use fune_model::design::setting_dims as dims;
use fune_model::resolve::ResolvedPart;
use sat_analysis::plate_grid::thinnest_axis;
use sat_analysis::structure::modal_fem::{
    ModalFemAxis, ModalFemFace, ModalFemInput, ModalFemLumped, ModalFemPanel, ModalFemSkipped,
    ModalFemSupport, ModalFemSupportError,
};
use sat_units::typed::{Length, Mass, Qty};
use std::collections::BTreeSet;

/// 機軸（打ち上げ方向の軸）。機体座標の z（`structure.launch_fit` の包絡と同じ約束）。
const LAUNCH_AXIS: ModalFemAxis = ModalFemAxis::Z;

/// 読んだパネルと、モデルにできなかったパネル。
struct Panels {
    panels: Vec<ModalFemPanel>,
    /// 軸に沿わず、モデルにしなかったパネル（質量は集中質量にする）
    skipped: Vec<(String, String)>,
}

/// `structure.modal_fem`: パネル・集中質量・フレームの質量・拘束と、設定 `[modal]`。
pub fn modal_fem(ctx: &Context<'_>) -> Result<ModalFemInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let Panels { panels, skipped } = read_panels(ctx, &mut m);
    if crate::parts::by_role(ctx, Role::Panel).next().is_none() {
        m.push(
            Target::any_of_roles(&[Role::Panel], None),
            "No panel part (role = panel)",
        );
    }
    let ties = Ties::read(ctx, &panels, &mut m);
    let lumped = lumped(ctx, &ties, &mut m);
    let support = support(ctx, &mut m);
    let settings = ctx
        .design
        .analysis
        .structure
        .as_ref()
        .and_then(|s| s.modal.as_ref());
    let mesh_size = setting_opt(
        &dims::STRUCTURE_MODAL_MESH_SIZE,
        &[],
        settings.and_then(|s| s.mesh_size.as_ref()),
        &mut m,
    );
    let merge_tolerance = setting_opt(
        &dims::STRUCTURE_MODAL_JOINT_TOLERANCE,
        &[],
        settings.and_then(|s| s.joint_tolerance.as_ref()),
        &mut m,
    );
    let distributed_mass = frame_mass(ctx, &mut m);
    m.finish(|| {
        Some(ModalFemInput {
            panels,
            lumped,
            skipped_panels: skipped
                .into_iter()
                .map(|(id, reason)| ModalFemSkipped { id, reason })
                .collect(),
            distributed_mass,
            launch_axis: LAUNCH_AXIS,
            support: support?,
            mesh_size,
            mode_count: None,
            merge_tolerance,
            drilling_factor: None,
        })
    })
}

/// パネル（role = panel）を機体座標の中央面にする。箱の薄い方向が法線で、箱の中心が中央面の中心。
/// 箱の軸が機体の軸に沿わないパネルは、理由つきで飛ばす（質量は [`lumped`] が集中質量にする）。
fn read_panels(ctx: &Context<'_>, m: &mut MissingInputs) -> Panels {
    let mut out = Panels {
        panels: Vec::new(),
        skipped: Vec::new(),
    };
    for p in crate::parts::by_role(ctx, Role::Panel) {
        let Some(read) = panel::read(ctx, p, m) else {
            continue;
        };
        let at = placed(ctx, p);
        let Some(extents) = axis_aligned_extents(ctx.resolved, &p.id, read.size) else {
            out.skipped.push((
                p.id.clone(),
                "The panel box is not aligned with the body axes; its mass is a lumped mass"
                    .to_owned(),
            ));
            continue;
        };
        let axes = [ModalFemAxis::X, ModalFemAxis::Y, ModalFemAxis::Z];
        let Some(normal) = ModalFemAxis::from_index(thinnest_axis(extents)) else {
            continue;
        };
        let in_plane: Vec<f64> = axes
            .into_iter()
            .filter(|a| *a != normal)
            .map(|a| extents[a.index()])
            .collect();
        out.panels.push(ModalFemPanel {
            id: p.id.clone(),
            center: [at.position.x, at.position.y, at.position.z].map(Qty::from_raw),
            normal,
            size: [in_plane[0], in_plane[1]].map(Qty::from_raw),
            thickness: read.thickness,
            youngs_modulus: read.youngs_modulus,
            poisson_ratio: read.poisson_ratio,
            density: read.density,
            // `mass` があれば質量行列だけがそれになる（肉抜きの表現）。剛性は密度と板厚から
            mass: ctx
                .scalar(p, "mass")
                .map(|kg| Qty::from_raw(extensive(p, kg))),
        });
    }
    out
}

/// 部品をどのパネルにつなぐかの決め方: 設定の上書き（`[modal].tie`）→ 取付先をたどって最初のパネル。
/// どちらも無ければ `None`（解析が最も近いパネルを選び、出力の `attachments` に出す）。
struct Ties {
    modelled: BTreeSet<String>,
    overrides: Vec<(String, String)>,
}

impl Ties {
    /// 設定を読む。パネルでないものを指していれば、その設定の欄を不足入力にする（黙って無視しない）。
    fn read(ctx: &Context<'_>, panels: &[ModalFemPanel], m: &mut MissingInputs) -> Self {
        let modelled: BTreeSet<String> = panels.iter().map(|p| p.id.clone()).collect();
        let tie = ctx
            .design
            .analysis
            .structure
            .as_ref()
            .and_then(|s| s.modal.as_ref())
            .map(|s| &s.tie);
        let mut overrides = Vec::new();
        for (part, panel) in tie.into_iter().flatten() {
            let path = AnalysisSystem::Structure
                .setting_path()
                .child(&["modal", "tie", part]);
            ctx.note(path.clone());
            if !ctx.resolved.parts.contains_key(part.as_str()) {
                m.push(path, format!("`{part}` is not a part of the design"));
            } else if !modelled.contains(panel) {
                m.push(
                    path,
                    format!("`{panel}` is not a panel of the model (a part with role = panel)"),
                );
            } else {
                overrides.push((part.clone(), panel.clone()));
            }
        }
        Self {
            modelled,
            overrides,
        }
    }

    /// 部品のつなぎ先（設定 → 取付先をたどって最初のパネル）。
    fn panel_of(&self, ctx: &Context<'_>, part: &str) -> Option<String> {
        if let Some((_, panel)) = self.overrides.iter().find(|(p, _)| p == part) {
            return Some(panel.clone());
        }
        ctx.resolved
            .mount_ancestors(part)
            .into_iter()
            .find(|a| self.modelled.contains(*a))
            .map(str::to_owned)
    }
}

/// 質量を持つ部品（パネルとフレームを除く。軸に沿わず飛ばしたパネルは含む）を、重心（部品の原点）の集中質量にする。
/// 外形（`size`）が機体の軸に沿っていれば、その投影の範囲に配る。質量は員数を掛けた素の質量。
fn lumped(ctx: &Context<'_>, ties: &Ties, m: &mut MissingInputs) -> Vec<ModalFemLumped> {
    ctx.resolved
        .parts
        .values()
        .filter(|p| p.role != Some(Role::Structure) && !ties.modelled.contains(&p.id))
        .filter_map(|p| {
            let mass = extensive(p, mass_or_missing(ctx, p, m)?);
            let at = placed(ctx, p);
            let footprint = at
                .size
                .and_then(|s| axis_aligned_extents(ctx.resolved, &p.id, [s.x, s.y, s.z]))
                .map(|e| e.map(Qty::<Length>::from_raw));
            Some(ModalFemLumped {
                id: p.id.clone(),
                position: [at.position.x, at.position.y, at.position.z].map(Qty::from_raw),
                mass: Qty::<Mass>::from_raw(mass),
                footprint,
                panel: ties.panel_of(ctx, &p.id),
            })
        })
        .collect()
}

/// フレーム（role = structure）の質量の合計。剛性は入れず、質量だけを全体に配る（解析の `distributed_mass`）。
fn frame_mass(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Qty<Mass>> {
    let frames: Vec<&ResolvedPart> = crate::parts::by_role(ctx, Role::Structure).collect();
    let total: f64 = frames
        .iter()
        .filter_map(|p| Some(extensive(p, mass_or_missing(ctx, p, m)?)))
        .sum();
    (total > 0.0).then(|| Qty::from_raw(total))
}

/// 打ち上げ時の拘束（属性 `launch_constraint`・`launch_constraint_face`。優先は `launch.toml` → ディスペンサー → 打ち上げ機）。
/// **面は、種類を出した出所だけから読む**（別の出所の面を混ぜて、誰も書いていない拘束を作らない）。
/// どこにも種類が無ければ足りない入力（黙った既定値は置かない）。綴りの意味は解析（`ModalFemSupport::from_launch_kind`）が持つ。
fn support(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<ModalFemSupport> {
    let values = LaunchValues::new(ctx);
    let launch = ctx.design.conditions.launch.as_ref();
    let set = values.text_set(&[
        (
            attr::CONSTRAINT,
            launch.and_then(|l| l.launch_constraint.as_deref()),
        ),
        (
            attr::CONSTRAINT_FACE,
            launch.and_then(|l| l.launch_constraint_face.as_deref()),
        ),
    ]);
    let Some(TextSet { origin, values }) = set else {
        m.push(
            launch_path(&[attr::CONSTRAINT]),
            format!(
                "Launch support type ({}): write `{}` in conditions/launch.toml or give the `{}` attribute to the dispenser or launcher",
                ModalFemSupport::LAUNCH_KINDS.join(", "),
                attr::CONSTRAINT,
                attr::CONSTRAINT,
            ),
        );
        return None;
    };
    let [Some(kind), face] = &values[..] else {
        return None;
    };
    let face = match face.as_deref().map(|f| (ModalFemFace::parse(f), f)) {
        Some((Some(face), _)) => Some(face),
        Some((None, f)) => {
            let reason = format!("`{f}` ({origin}) is not a face (one of {})", face_names());
            m.push(launch_path(&[attr::CONSTRAINT_FACE]), reason);
            return None;
        }
        None => None,
    };
    match ModalFemSupport::from_launch_kind(kind, face) {
        Ok(support) => Some(support),
        Err(e) => {
            let (at, reason) = unusable_support(e, kind, &origin);
            m.push(at, reason);
            None
        }
    }
}

/// 拘束を作れなかった理由を、書き直す欄と文にする。
fn unusable_support(
    e: ModalFemSupportError,
    kind: &str,
    origin: &str,
) -> (fune_model::path::SettingPath, String) {
    match e {
        ModalFemSupportError::FaceRequired => (
            launch_path(&[attr::CONSTRAINT_FACE]),
            format!(
                "Face of the launch support `{}` ({}), one of {}: write `{}` in conditions/launch.toml or give the `{}` attribute to the dispenser or launcher",
                kind,
                origin,
                face_names(),
                attr::CONSTRAINT_FACE,
                attr::CONSTRAINT_FACE,
            ),
        ),
        ModalFemSupportError::UnknownKind => (
            launch_path(&[attr::CONSTRAINT]),
            format!(
                "`{}` ({}) is not a launch support type (one of {})",
                kind,
                origin,
                ModalFemSupport::LAUNCH_KINDS.join(", ")
            ),
        ),
    }
}

fn face_names() -> String {
    ModalFemFace::ALL.map(ModalFemFace::as_str).join(", ")
}
