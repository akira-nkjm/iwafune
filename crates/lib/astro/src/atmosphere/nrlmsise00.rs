//! NRLMSISE-00 経験大気モデル（0〜1000 km の中性大気）。
//!
//! ## これは何か
//!
//! [`super::density_exponential_kg_m3`] の区分指数モデルは**静的**で、高度しか見ない。実大気は太陽活動
//! （F10.7）・地磁気活動（Ap）・季節・緯度経度・地方時で数倍〜1 桁変わる。NRLMSISE-00 は
//! それらに応答する経験モデルで、地表から下部外気圏までの中性大気の事実上の標準である。
//!
//! ## 原典と移植方針
//!
//! Dominik Brodowski の C 実装（NRL の FORTRAN 版 NRLMSISE-00 からの移植。Picone, Hedin,
//! Drob 2002）を 1 対 1 で Rust へ移したもの。**式も評価順序も変えていない。** 変えると原典の
//! 検証出力とビットレベルで合わなくなり、突き合わせ（`tests`）が「移植の誤り」と
//! 「並べ替えによる丸め差」を区別できなくなるため。関数名（`gtd7` / `gts7` / `globe7` / `densu` …）と
//! 係数の添字も原典のままにしてあり、原典と並べて読める。係数 3,240 個は `coeffs` にある。
//!
//! ## 検証
//!
//! 原典に付属する 17 ケースの検証出力を `data/nrlmsise00_reference.txt` に固定し、`tests` で突き合わせる
//! （相対差 1e-14 以内）。
//!
//! ## 適用範囲と範囲外の扱い
//!
//! モデルの妥当範囲は 0〜1000 km。**範囲外で打ち切らない**——ある高度で密度を 0 に切ると加速度が不連続になり、
//! 適応刻みの積分器が刻みを最小値まで詰め続ける。1000 km 超では Bates 温度プロファイルが滑らかに外挿され、
//! 密度は指数的に減り続ける。地表下は 0 km にクランプする（[`Nrlmsise00Input::from_geodetic`]）。

mod coeffs;
mod gtd7;
mod harmonics;
mod profile;
mod thermosphere;

/// 地磁気活動の与え方。
///
/// 原典のスイッチ 9 に対応する。日平均 Ap だけを使うのが標準（`sw[9] = 1`）で、
/// 磁気嵐の応答を細かく見たいときは直前 57 時間の 3 時間 Ap 履歴を使う（`sw[9] = -1`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeomagneticActivity {
    /// 日平均 Ap 指数のみ（標準）。
    DailyAp(f64),
    /// 3 時間 Ap の履歴 7 要素。原典 `ap_array` と同じ並び:
    /// `[日平均, 現時点, -3h, -6h, -9h, -12〜-33h 平均, -36〜-57h 平均]`。
    ThreeHourlyAp([f64; 7]),
}

/// 宇宙天気の入力（太陽 F10.7 と地磁気 Ap）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceWeather {
    /// 前日の F10.7 太陽電波フラックス `[sfu]`。
    pub f107_daily: f64,
    /// F10.7 の 81 日中心移動平均 `[sfu]`。
    pub f107_81day_avg: f64,
    /// 地磁気活動。
    pub geomagnetic: GeomagneticActivity,
}

impl SpaceWeather {
    /// 中程度の太陽活動・静穏な地磁気という慣用的な既定値（F10.7 = 150, Ap = 4）。
    ///
    /// 原典の検証ケースが使う値でもある。実運用では観測値・予測値を渡すこと。
    #[must_use]
    pub const fn nominal() -> Self {
        Self {
            f107_daily: 150.0,
            f107_81day_avg: 150.0,
            geomagnetic: GeomagneticActivity::DailyAp(4.0),
        }
    }
}

/// NRLMSISE-00 の入力 1 点。単位は SI（高度は m、角度は度）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nrlmsise00Input {
    /// 年間通日 `[1..=366]`。
    pub day_of_year: u16,
    /// UT の 0 時からの経過秒 `[s]`。
    pub seconds_of_day_ut: f64,
    /// 測地高度 `[m]`。
    pub altitude_m: f64,
    /// 測地緯度 `[deg]`。
    pub latitude_deg: f64,
    /// 経度 `[deg]`（東経正、-180〜180 でも 0〜360 でもよい）。
    pub longitude_deg: f64,
    /// 地方視太陽時 `[h]`。
    pub local_solar_time_h: f64,
    /// 宇宙天気。
    pub weather: SpaceWeather,
}

impl Nrlmsise00Input {
    /// 測地座標と時刻から入力を組み立てる。
    ///
    /// 地方太陽時は原典の推奨どおり `sec/3600 + lon/15` で求める（原典 `nrlmsise-00.h` の
    /// 注記: 経度と地方時を独立に与えると非物理な組み合わせになりうるので、通常はこの関係で
    /// 与える）。高度は地表下を 0 にクランプする——モデルは地表より下を持たず、外挿すると
    /// 温度スプラインが破綻するため。
    #[must_use]
    pub fn from_geodetic(
        day_of_year: u16,
        seconds_of_day_ut: f64,
        altitude_m: f64,
        latitude_deg: f64,
        longitude_deg: f64,
        weather: SpaceWeather,
    ) -> Self {
        let local_solar_time_h = seconds_of_day_ut / 3600.0 + longitude_deg / 15.0;
        Self {
            day_of_year,
            seconds_of_day_ut,
            altitude_m: altitude_m.max(0.0),
            latitude_deg,
            longitude_deg,
            local_solar_time_h,
            weather,
        }
    }
}

/// NRLMSISE-00 の出力。数密度は `m⁻³`、質量密度は `kg/m³`、温度は `K`。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Nrlmsise00Output {
    /// 全質量密度 `[kg/m³]`。**異常酸素を含まない**（原典 `gtd7` の定義）。
    /// 力モデル（[`crate::force::earth_drag_eci`]）が使うのはこれ。
    pub total_mass_density_kg_m3: f64,
    /// He 数密度 `[m⁻³]`。
    pub he_m3: f64,
    /// O 数密度 `[m⁻³]`。
    pub o_m3: f64,
    /// N₂ 数密度 `[m⁻³]`。
    pub n2_m3: f64,
    /// O₂ 数密度 `[m⁻³]`。
    pub o2_m3: f64,
    /// Ar 数密度 `[m⁻³]`。
    pub ar_m3: f64,
    /// H 数密度 `[m⁻³]`。
    pub h_m3: f64,
    /// N 数密度 `[m⁻³]`。
    pub n_m3: f64,
    /// 異常酸素の数密度 `[m⁻³]`（高高度で効く成分。全質量密度には含めていない）。
    pub anomalous_o_m3: f64,
    /// 外気圏温度 `T∞` `[K]`。
    pub exospheric_temperature_k: f64,
    /// その高度の温度 `[K]`。
    pub temperature_k: f64,
}

/// 原典の内部表現に合わせた入力（高度 km、角度 度、CGS 系）。
#[derive(Debug, Clone, Copy)]
struct Input {
    day_of_year: u16,
    sec_ut: f64,
    altitude_km: f64,
    latitude_deg: f64,
    longitude_deg: f64,
    local_solar_time_h: f64,
    f107_daily: f64,
    f107_81day_avg: f64,
    ap_daily: f64,
    ap_array: [f64; 7],
}

/// 原典 `struct nrlmsise_output` に対応する CGS の生出力。
#[derive(Debug, Clone, Copy, Default)]
struct RawOutput {
    /// `[He, O, N2, O2, Ar, 全質量, H, N, 異常酸素]`。数密度は `cm⁻³`、質量密度は `g/cm³`。
    d: [f64; 9],
    /// 外気圏温度 `[K]`。
    t_inf: f64,
    /// その高度の温度 `[K]`。
    t_alt: f64,
}

/// 原典が静的変数で持ち回している状態。
///
/// 原典はこれらをファイルスコープの `static` に置き、`globe7` が書いて `glob7s` が読む、
/// `gts7` が書いて `gtd7` が読む、という形で共有している。Rust では 1 回の評価に閉じた
/// 構造体にして持ち回る——**グローバル可変状態にしないことで評価が純関数になり、
/// 並列伝播でも決定論が保たれる**。
#[derive(Debug, Clone, Default)]
struct State {
    /// 地表重力 `[cm/s²]`（原典 PARMB）。
    gsurf: f64,
    /// 実効地球半径 `[km]`（原典 PARMB）。
    re: f64,
    /// N₂ の完全混合密度（原典 DMIX の `dm28`）。
    dm28: f64,
    /// 温度ノードと端点勾配（原典 MESO7）。
    meso_tn1: [f64; 5],
    meso_tn2: [f64; 4],
    meso_tn3: [f64; 5],
    meso_tgn1: [f64; 2],
    meso_tgn2: [f64; 2],
    meso_tgn3: [f64; 2],
    /// `F10.7A - 150`（原典 LPOLY）。`globe7` が書き `glob7s` が読む。
    dfa: f64,
    /// Legendre 多項式（原典 LPOLY の `plg[4][9]`）。
    plg: [[f64; 9]; 4],
    /// 地方時の三角関数（原典 LPOLY）。
    ctloc: f64,
    stloc: f64,
    c2tloc: f64,
    s2tloc: f64,
    c3tloc: f64,
    s3tloc: f64,
    /// 日平均 ap 由来の地磁気項（原典 LPOLY の `apdf`）。
    apdf: f64,
    /// 3 時間 ap 由来の地磁気項（原典 LPOLY の `apt[0]`）。
    apt: f64,
    /// 原典 `tselec` が作る変動スイッチ。
    sw: [f64; 24],
    swc: [f64; 24],
}

/// NRLMSISE-00 を 1 点評価する。
///
/// 純関数（内部状態を持たない）。原典 `gtd7` に対応し、全質量密度は**異常酸素を含まない**。
///
/// # 前提: `altitude_m >= 0`
///
/// 地表下では原典の下層大気の式が発散し、−50 km で `2.5e35 g/cm³`、−100 km で `inf` を返す。
/// **原典 C も同じ**なので移植の誤りではなく、ここで丸めて直すと突き合わせの意味が失われる。
///
/// [`Nrlmsise00Input::from_geodetic`] は `max(0.0)` でクランプするので、この関数を
/// [`super::AtmosphereModel`] 経由で使う限り踏めない（積分器は届かない）。
/// 構造体リテラルで直に組むときだけ、呼び出し側が高度を保証すること。
///
/// # Examples
///
/// ```
/// use sat_astro::atmosphere::nrlmsise00::{evaluate, Nrlmsise00Input, SpaceWeather};
///
/// // 2000-06-20(通日 172) 08:03:20 UT, 高度 400 km, 北緯 60 度, 西経 70 度
/// let input = Nrlmsise00Input::from_geodetic(
///     172, 29_000.0, 400_000.0, 60.0, -70.0, SpaceWeather::nominal(),
/// );
/// let out = evaluate(&input);
/// assert!(out.total_mass_density_kg_m3 > 1e-13);
/// assert!(out.total_mass_density_kg_m3 < 1e-11);
/// ```
#[must_use]
pub fn evaluate(input: &Nrlmsise00Input) -> Nrlmsise00Output {
    evaluate_raw(input).into_si()
}

/// 公開入力を原典の内部表現へ直して `gtd7` を回す。
fn evaluate_raw(input: &Nrlmsise00Input) -> RawOutput {
    let (ap_daily, ap_array) = match input.weather.geomagnetic {
        GeomagneticActivity::DailyAp(ap) => (ap, [ap; 7]),
        GeomagneticActivity::ThreeHourlyAp(array) => (array[0], array),
    };
    let raw_input = Input {
        day_of_year: input.day_of_year,
        sec_ut: input.seconds_of_day_ut,
        altitude_km: input.altitude_m / 1000.0,
        latitude_deg: input.latitude_deg,
        longitude_deg: input.longitude_deg,
        local_solar_time_h: input.local_solar_time_h,
        f107_daily: input.weather.f107_daily,
        f107_81day_avg: input.weather.f107_81day_avg,
        ap_daily,
        ap_array,
    };
    State::new(input.weather.geomagnetic).gtd7(&raw_input)
}

impl RawOutput {
    /// CGS（`cm⁻³` / `g/cm³`）から SI（`m⁻³` / `kg/m³`）へ直す。
    fn into_si(self) -> Nrlmsise00Output {
        /// `cm⁻³` → `m⁻³`。
        const NUMBER_DENSITY: f64 = 1.0E6;
        /// `g/cm³` → `kg/m³`。
        const MASS_DENSITY: f64 = 1.0E3;
        Nrlmsise00Output {
            total_mass_density_kg_m3: self.d[5] * MASS_DENSITY,
            he_m3: self.d[0] * NUMBER_DENSITY,
            o_m3: self.d[1] * NUMBER_DENSITY,
            n2_m3: self.d[2] * NUMBER_DENSITY,
            o2_m3: self.d[3] * NUMBER_DENSITY,
            ar_m3: self.d[4] * NUMBER_DENSITY,
            h_m3: self.d[6] * NUMBER_DENSITY,
            n_m3: self.d[7] * NUMBER_DENSITY,
            anomalous_o_m3: self.d[8] * NUMBER_DENSITY,
            exospheric_temperature_k: self.t_inf,
            temperature_k: self.t_alt,
        }
    }
}

/// 原典と同じ CGS の生出力を返す（検証専用）。
///
/// `tests` が原典の検証出力（CGS）と**単位換算を挟まずに**
/// 突き合わせるために使う。換算を挟むと、移植の誤りと換算の誤りが混ざって切り分けられない。
#[cfg(test)]
fn evaluate_cgs_for_tests(input: &Nrlmsise00Input) -> ([f64; 9], f64, f64) {
    let raw = evaluate_raw(input);
    (raw.d, raw.t_inf, raw.t_alt)
}

#[cfg(test)]
mod tests;
