//! 軌道条件の解析。物理モデルは sat-astro、季節の標本と要約・高度低下の積分はここが持つ。
pub mod decay;
pub mod eclipse_season;
pub mod sun_distance;
pub use decay::{OrbitDecay, OrbitDecayInput, OrbitDecayOutput};
pub use eclipse_season::{
    EclipseSeason, EclipseSeasonInput, EclipseSeasonOutput, EclipseSeasonPoint,
};
pub use sun_distance::{
    SunDistanceExtremes, SunDistanceInput, SunDistancePoint, sun_distance_extremes,
};
