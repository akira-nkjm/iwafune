//! 軌道条件の解析。物理モデルは sat-astro、季節の標本と要約はここが持つ。
pub mod eclipse_season;
pub use eclipse_season::{
    EclipseSeason, EclipseSeasonInput, EclipseSeasonOutput, EclipseSeasonPoint,
};
