//! パネル（role = panel）の読み出し。**パネルの材料・外形・板厚の読み方はここだけ**
//! （`structure.modal_estimate` の簡易推定と `structure.modal_fem` の有限要素が共有する）。足りないものは不足入力に記録する。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::setting;
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::{AnalysisSystem, MaterialSettings};
use fune_model::resolve::{ResolvedPart, SiValue};
use sat_units::typed::{Length, MassDensity, Pressure, Qty};

/// 読んだパネル 1 枚（材料の物性・外形・板厚）。
#[derive(Debug, Clone)]
pub(super) struct PanelRead {
    /// 箱の辺の長さ（部品座標系。薄い方向を含む）[m]
    pub size: [f64; 3],
    pub thickness: Qty<Length>,
    pub youngs_modulus: Qty<Pressure>,
    pub poisson_ratio: f64,
    pub density: Qty<MassDensity>,
}

/// 部品の材料（属性 `material` の名前と、`analysis.structure.material.<名前>` の物性）。**部品の材料の引き方はここだけ**
/// （パネルの読み出しと、熱の板の面内の伝導が共有する）。無ければ不足入力に記録して `None`。
pub(crate) fn material<'a>(
    ctx: &'a Context<'_>,
    p: &ResolvedPart,
    m: &mut MissingInputs,
) -> Option<(String, &'a MaterialSettings)> {
    let Some(SiValue::Enum(name)) = ctx.attr(p, "material") else {
        m.push(Target::attr(&p.id, "material"), "Material");
        return None;
    };
    let materials = ctx.design.analysis.structure.as_ref().map(|s| &s.material);
    let mp = AnalysisSystem::Structure
        .setting_path()
        .child(&["material", name]);
    let mat = m.need(
        materials.and_then(|ms| ms.get(name.as_str())),
        mp,
        "Material properties",
    )?;
    Some((name.clone(), mat))
}

/// パネル 1 枚を読む（材料は `analysis.structure.material`、外形は `size`、板厚は `thickness`）。
/// どれかが無ければ、読めたものまで全部を不足入力に記録して `None`。
pub(super) fn read(
    ctx: &Context<'_>,
    p: &ResolvedPart,
    m: &mut MissingInputs,
) -> Option<PanelRead> {
    let (name, mat) = material(ctx, p, m)?;
    let mp = AnalysisSystem::Structure
        .setting_path()
        .child(&["material", &name]);
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
    Some(PanelRead {
        size: size?,
        thickness: thickness?,
        youngs_modulus: e?,
        poisson_ratio: nu?,
        density: rho?,
    })
}
