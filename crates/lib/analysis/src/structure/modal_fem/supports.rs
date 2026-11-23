//! 拘束。種類（`base_fixed`・`rails`・`tabs`・`fixed`・`free`）を、固定する自由度の一覧に直す。
//! 面・辺は、節点全体を囲む箱（外接の箱）の面・辺で、許容差の内にある節点を選ぶ。

use super::input::{
    ModalFemAxis, ModalFemFace, ModalFemFixedItem, ModalFemSupport, ModalFemTarget,
};
use super::mesh::Mesh;
use crate::spec::AnalysisError;

/// 節点の全自由度（並進 3 + 回転 3）。
const ALL: [usize; 6] = [0, 1, 2, 3, 4, 5];
/// 並進の 3 自由度。
const TRANSLATIONS: [usize; 3] = [0, 1, 2];

/// 拘束の選び方に必要なもの。
struct Selector<'a> {
    mesh: &'a Mesh,
    lo: [f64; 3],
    hi: [f64; 3],
    tol: f64,
    fixed: Vec<bool>,
}

impl Selector<'_> {
    fn on_face(&self, node: usize, face: ModalFemFace) -> bool {
        let a = face.axis().index();
        let extreme = if face.is_max() {
            self.hi[a]
        } else {
            self.lo[a]
        };
        (self.mesh.coords[node][a] - extreme).abs() <= self.tol
    }

    /// 軸 `a` の外接の箱の最小側か最大側にあるか。
    fn on_either(&self, node: usize, a: usize) -> bool {
        let c = self.mesh.coords[node][a];
        (c - self.lo[a]).abs() <= self.tol || (c - self.hi[a]).abs() <= self.tol
    }

    /// 条件に合う節点の `comps` を固定する。合う節点が無ければ入力の誤り。
    fn fix_where(
        &mut self,
        what: &str,
        comps: &[usize],
        pred: impl Fn(&Self, usize) -> bool,
    ) -> Result<(), AnalysisError> {
        let nodes: Vec<usize> = (0..self.mesh.node_count())
            .filter(|n| pred(self, *n))
            .collect();
        if nodes.is_empty() {
            return Err(AnalysisError::InvalidInput(format!(
                "Support {what} selects no nodes"
            )));
        }
        for n in nodes {
            for c in comps {
                self.fixed[n * 6 + c] = true;
            }
        }
        Ok(())
    }

    fn fix_item(&mut self, item: &ModalFemFixedItem) -> Result<(), AnalysisError> {
        let comps: &[usize] = if item.clamp { &ALL } else { &TRANSLATIONS };
        match &item.target {
            ModalFemTarget::Face { face } => {
                self.fix_where("face", comps, |s, n| s.on_face(n, *face))
            }
            ModalFemTarget::Edge { faces } => {
                if faces[0].axis() == faces[1].axis() {
                    return Err(AnalysisError::InvalidInput(
                        "An edge needs two faces on different axes".to_owned(),
                    ));
                }
                self.fix_where("edge", comps, |s, n| {
                    s.on_face(n, faces[0]) && s.on_face(n, faces[1])
                })
            }
            ModalFemTarget::Point { position } => {
                let p = position.map(|x| x.value());
                let d2 = |n: usize| -> f64 {
                    (0..3)
                        .map(|a| (self.mesh.coords[n][a] - p[a]).powi(2))
                        .sum()
                };
                let best = (0..self.mesh.node_count()).min_by(|a, b| d2(*a).total_cmp(&d2(*b)));
                self.fix_where("point", comps, |_, n| Some(n) == best)
            }
        }
    }

    /// レール: 機軸に平行な 4 本の辺の横方向と、8 つの角の機軸方向。
    fn rails(&mut self, axis: ModalFemAxis) -> Result<(), AnalysisError> {
        let a = axis.index();
        let (b, c) = ((a + 1) % 3, (a + 2) % 3);
        let on_line = |s: &Self, n: usize| s.on_either(n, b) && s.on_either(n, c);
        self.fix_where("rails (edges)", &[b, c], on_line)?;
        self.fix_where("rails (corners)", &[a], |s, n| {
            on_line(s, n) && s.on_either(n, a)
        })
    }

    /// タブ: 面の、機軸に平行な 2 本の辺の並進。
    fn tabs(&mut self, face: ModalFemFace, axis: ModalFemAxis) -> Result<(), AnalysisError> {
        let n_axis = face.axis().index();
        if n_axis == axis.index() {
            return Err(AnalysisError::InvalidInput(
                "Tabs need a face parallel to the launch axis".to_owned(),
            ));
        }
        let t = 3 - n_axis - axis.index();
        self.fix_where("tabs", &TRANSLATIONS, |s, n| {
            s.on_face(n, face) && s.on_either(n, t)
        })
    }
}

/// 拘束で固定する自由度（全体の自由度 = 節点 × 6 + 成分。`true` が固定）。
pub fn fixed_dofs(
    mesh: &Mesh,
    support: &ModalFemSupport,
    launch_axis: ModalFemAxis,
    tol: f64,
) -> Result<Vec<bool>, AnalysisError> {
    let (lo, hi) = mesh.bounds();
    let mut s = Selector {
        mesh,
        lo,
        hi,
        tol,
        fixed: vec![false; mesh.node_count() * 6],
    };
    match support {
        ModalFemSupport::Free => {}
        ModalFemSupport::BaseFixed { face } => {
            s.fix_where("base_fixed", &ALL, |s, n| s.on_face(n, *face))?;
        }
        ModalFemSupport::Rails => s.rails(launch_axis)?,
        ModalFemSupport::Tabs { face } => s.tabs(*face, launch_axis)?,
        ModalFemSupport::Fixed { items } => {
            for item in items {
                s.fix_item(item)?;
            }
        }
    }
    Ok(s.fixed)
}
