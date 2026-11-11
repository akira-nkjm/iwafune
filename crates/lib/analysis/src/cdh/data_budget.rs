//! データ量とストレージ（`cdh.data_budget`）。HK・ミッションデータの 1 日の発生量、保持に必要な量、ストレージ余裕。
//!
//! - 部品のデータレートは状態ごと（[`StateValues`]、単体値）。モードの状態と時間割合は [`crate::modes`] で求める。
//! - 1 日の発生量 = Σ_モード（1 日に占める割合 × Σ_部品 平均レート）× 86400 s。
//! - 保持に必要な量 = 1 日の発生量 × 保持期間 / 1 日（地上に降ろせない最悪の期間ぶん溜める）。
//! - 部品のレートには、HK はパケットの余分 `(1 + hk_overhead)` を、ミッションデータは圧縮率 `compression_ratio` を掛ける。

use crate::budget::{BudgetItem, counted_total, limit_check};
use crate::modes::{DUTY_TOLERANCE, ModeDef, StateValues, UnlistedParts, evaluate, find_mode};
use crate::spec::quantity_text;
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Check, System, Verdict, ensure_fraction,
    ensure_non_negative, ensure_positive, ratio_margin, ratio_margin_verdict,
};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_units::typed::{DataRate, Fraction, Information, Qty, Time};
use serde::{Deserialize, Serialize};

/// ストレージの容量のうち残しておく割合の既定値。**既定値はここだけ。**
pub const DEFAULT_STORAGE_RESERVE: Fraction = Fraction::new(0.2);

/// データを出す部品。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetPart {
    /// 員数。レートは単体値を渡す（省略時 1）。
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 部品の ID（モードの割り当ての `part` と同じ）
    pub id: String,
    /// HK（テレメトリ）のレート（1 台の状態ごと）
    #[serde(default)]
    pub hk_rate: StateValues<DataRate>,
    /// ミッションデータのレート（1 台の状態ごと）
    #[serde(default)]
    pub mission_rate: StateValues<DataRate>,
}

/// 搭載ストレージ（部品 1 つ。容量 × 員数を足し合わせる）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetStorage {
    pub part: String,
    /// 1 台あたりの容量
    pub capacity: Qty<Information>,
    /// 員数（省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
}

/// 1 日に占めるモードの割合。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetModeShare {
    pub mode: String,
    /// 0〜1。合計は 1 以下（足りない分はデータを出さない時間とみなし、警告する）
    pub fraction: f64,
}

/// データ量とストレージの入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetInput {
    pub parts: Vec<DataBudgetPart>,
    pub modes: Vec<ModeDef>,
    pub mode_shares: Vec<DataBudgetModeShare>,
    /// 搭載ストレージ（容量はここで足し合わせる）
    pub storage: Vec<DataBudgetStorage>,
    /// 地上に降ろせずに保持する期間（パスの間隔の最悪値や運用の保持期間）
    pub retention: Qty<Time>,
    /// 1 日に降ろせる量（`comm.data_throughput` の `capacity_per_day` など。与えると収支を判定する）
    #[serde(default)]
    pub downlink_per_day: Option<Qty<Information>>,
    /// ストレージの容量のうち残しておく割合（0 以上 1 未満、既定 [`DEFAULT_STORAGE_RESERVE`]）。
    /// 必要量がこれを食い込むと警告、容量を超えると違反
    #[serde(default)]
    pub storage_reserve: Option<Fraction>,
    /// ミッションデータの圧縮率（圧縮後 / 圧縮前、既定 1）
    #[serde(default)]
    pub compression_ratio: Option<f64>,
    /// HK に掛けるパケットの余分（割合、既定 0）
    #[serde(default)]
    pub hk_overhead: Option<f64>,
}

impl DataBudgetInput {
    /// 残しておく割合（省略時 [`DEFAULT_STORAGE_RESERVE`]）。
    pub fn storage_reserve(&self) -> Fraction {
        self.storage_reserve.unwrap_or(DEFAULT_STORAGE_RESERVE)
    }

    /// ミッションデータの圧縮率（省略時 1 = 圧縮しない）。
    pub fn compression_ratio(&self) -> f64 {
        self.compression_ratio.unwrap_or(1.0)
    }

    /// HK に掛けるパケットの余分（省略時 0）。
    pub fn hk_overhead(&self) -> f64 {
        self.hk_overhead.unwrap_or(0.0)
    }
}

/// 容量の余裕（`容量 / 必要量 − 1`）の要求値: 容量の `reserve` を残すなら `reserve / (1 − reserve)`
/// （例: 2 割を残す → 必要量は容量の 0.8 倍まで → 余裕 0.25 以上）。
pub fn required_capacity_margin(reserve: Fraction) -> Fraction {
    Fraction::new(reserve.value() / (1.0 - reserve.value()))
}

/// モードごとの平均レート。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetModeResult {
    pub mode: String,
    pub fraction: Fraction,
    pub hk_rate: Qty<DataRate>,
    pub mission_rate: Qty<DataRate>,
}

/// 部品ごとの 1 日の発生量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetPartResult {
    pub part: String,
    pub hk_per_day: Qty<Information>,
    pub mission_per_day: Qty<Information>,
    /// 全体の発生量に占める割合（0〜1）
    pub share: Fraction,
}

/// データ量とストレージの出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataBudgetOutput {
    pub hk_per_day: Qty<Information>,
    pub mission_per_day: Qty<Information>,
    pub total_per_day: Qty<Information>,
    /// 1 日平均のデータレート
    pub average_rate: Qty<DataRate>,
    pub modes: Vec<DataBudgetModeResult>,
    pub parts: Vec<DataBudgetPartResult>,
    /// 搭載ストレージの容量の合計（容量 × 員数）
    pub storage_capacity: Qty<Information>,
    /// 使った「容量のうち残しておく割合」（入力が無ければ既定値）
    pub storage_reserve: Fraction,
    /// 容量の余裕の要求値 = 残す割合 / (1 − 残す割合)（[`required_capacity_margin`]。これを下回ると警告）
    pub required_capacity_margin: Fraction,
    /// 保持に必要な量 = total_per_day × retention / 1 日
    pub storage_required: Qty<Information>,
    /// 容量の余裕 = 容量 / 必要量 − 1（[`crate::spec::ratio_margin`]。負なら足りない。必要量が 0 なら `None`）
    pub capacity_margin: Option<Fraction>,
    /// 1 日の収支 = 降ろせる量 − 発生量（`downlink_per_day` を与えたときだけ）
    pub downlink_balance_per_day: Option<Qty<Information>>,
    pub checks: Vec<Check>,
}

/// データ量とストレージ。
pub struct DataBudget;

impl Analysis for DataBudget {
    const ID: &'static str = "cdh.data_budget";
    const TITLE: &'static str = "データ量とストレージ";
    const SYSTEM: System = System::Cdh;
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "HK・ミッションデータの発生量と、保持が必要な量・ストレージ余裕";
    const TITLE_EN: &'static str = "Data volume and storage";
    const DESCRIPTION_EN: &'static str =
        "HK and mission data generation, required retention and storage margin";
    type Input = DataBudgetInput;
    type Output = DataBudgetOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let (share_sum, p) = validate(input)?;
        let mut warnings = Vec::new();
        let mut modes = Vec::with_capacity(input.mode_shares.len());
        // 部品ごとの (HK, ミッション) の 1 日平均レート [bit/s]
        let mut part_rates = vec![(0.0, 0.0); input.parts.len()];
        for share in &input.mode_shares {
            let mode = find_mode(&input.modes, &share.mode)?;
            let (rates, checks) = mode_rates(mode, &input.parts)?;
            warnings.extend(checks);
            let (mut hk, mut mission) = (0.0, 0.0);
            for (i, r) in rates.iter().enumerate() {
                let (h, m) = (r[0] * p.hk_factor, r[1] * p.compression);
                part_rates[i].0 += share.fraction * h;
                part_rates[i].1 += share.fraction * m;
                hk += h;
                mission += m;
            }
            modes.push(DataBudgetModeResult {
                mode: share.mode.clone(),
                fraction: Fraction::new(share.fraction),
                hk_rate: Qty::from_raw(hk),
                mission_rate: Qty::from_raw(mission),
            });
        }
        build_output(input, &p, share_sum, modes, &part_rates, warnings)
    }
}

/// 既定値を埋めた係数と、足し合わせたストレージ容量。
struct Params {
    storage_bits: f64,
    reserve: Fraction,
    compression: f64,
    hk_factor: f64,
}

/// ストレージの容量の合計 Σ 容量 × 員数（`budget::counted_total` で数える）。
fn storage_total(storage: &[DataBudgetStorage]) -> Result<f64, AnalysisError> {
    let items: Vec<BudgetItem> = storage
        .iter()
        .map(|s| BudgetItem::counted(s.part.clone(), s.capacity.value(), s.count))
        .collect();
    counted_total(&items)
}

/// 入力を検査し、モードの割合の合計と係数を返す。
fn validate(input: &DataBudgetInput) -> Result<(f64, Params), AnalysisError> {
    let p = Params {
        storage_bits: storage_total(&input.storage)?,
        reserve: input.storage_reserve(),
        compression: input.compression_ratio(),
        hk_factor: 1.0 + input.hk_overhead(),
    };
    ensure_positive("ストレージ容量の合計", p.storage_bits)?;
    ensure_fraction("ストレージに残す割合", p.reserve.value())?;
    // 1 だと使える量が 0 になる（バジェットのシステム余裕と同じ範囲）
    if p.reserve.value() >= 1.0 {
        return Err(AnalysisError::InvalidInput(
            "ストレージに残す割合は 1 未満にします".to_owned(),
        ));
    }
    ensure_non_negative("保持期間", input.retention.value())?;
    ensure_non_negative("圧縮率", p.compression)?;
    ensure_non_negative("HK の余分", p.hk_factor - 1.0)?;
    for s in &input.mode_shares {
        ensure_fraction(&format!("モード {} の割合", s.mode), s.fraction)?;
    }
    let sum: f64 = input.mode_shares.iter().map(|s| s.fraction).sum();
    if sum > 1.0 + DUTY_TOLERANCE {
        return Err(AnalysisError::InvalidInput(
            "モードの割合の合計が 1 を超えています".to_owned(),
        ));
    }
    Ok((sum, p))
}

/// 部品のレートの種類（HK とミッションデータ）。
type Pick = fn(&DataBudgetPart) -> &StateValues<DataRate>;

/// モード中の部品ごとの [HK, ミッション] の平均レート [bit/s]（係数を掛ける前）と、合計から除いた部品の警告。
/// 平均と除外の扱いは `modes::evaluate`。レートを持たない部品は渡さないので、モードにはあるがレートの一覧に
/// 無い部品は C&DH では正常として警告にしない（[`UnlistedParts::Ignore`]）。
fn mode_rates(
    mode: &ModeDef,
    parts: &[DataBudgetPart],
) -> Result<(Vec<[f64; 2]>, Vec<Check>), AnalysisError> {
    let kinds: [(&str, &str, Pick); 2] = [
        ("hk", "HK レート", |p| &p.hk_rate),
        ("mission", "ミッションデータのレート", |p| {
            &p.mission_rate
        }),
    ];
    let mut rates = vec![[0.0; 2]; parts.len()];
    let mut checks = Vec::new();
    for (k, (key, label, pick)) in kinds.into_iter().enumerate() {
        let ids = parts
            .iter()
            .filter(|p| !pick(p).is_empty())
            .map(|p| p.id.as_str());
        let ev = evaluate(mode, ids, UnlistedParts::Ignore, |id, state| {
            parts
                .iter()
                .find(|p| p.id == id)
                .and_then(|p| pick(p).get(state))
                .map(Qty::value)
        })?;
        for (id, mean) in &ev.means {
            if let (Some(i), Some(m)) = (parts.iter().position(|p| &p.id == id), mean) {
                rates[i][k] = *m * f64::from(parts[i].count);
            }
        }
        checks.extend(ev.checks(label).into_iter().map(|mut c| {
            c.key = format!("{key}:{}:{}", c.key, mode.id);
            c
        }));
    }
    Ok((rates, checks))
}

fn build_output(
    input: &DataBudgetInput,
    p: &Params,
    share_sum: f64,
    modes: Vec<DataBudgetModeResult>,
    part_rates: &[(f64, f64)],
    mut checks: Vec<Check>,
) -> Result<DataBudgetOutput, AnalysisError> {
    let hk: f64 = part_rates.iter().map(|r| r.0).sum::<f64>() * SECONDS_PER_DAY;
    let mission: f64 = part_rates.iter().map(|r| r.1).sum::<f64>() * SECONDS_PER_DAY;
    let total = hk + mission;
    let parts: Vec<DataBudgetPartResult> = input
        .parts
        .iter()
        .zip(part_rates)
        .map(|(p, &(h, m))| DataBudgetPartResult {
            part: p.id.clone(),
            hk_per_day: Qty::from_raw(h * SECONDS_PER_DAY),
            mission_per_day: Qty::from_raw(m * SECONDS_PER_DAY),
            share: Fraction::new(if total > 0.0 {
                (h + m) * SECONDS_PER_DAY / total
            } else {
                0.0
            }),
        })
        .collect();
    let required = total * input.retention.value() / SECONDS_PER_DAY;
    let margin = ratio_margin(p.storage_bits, required);
    let balance = input.downlink_per_day.map(|d| d.value() - total);
    let downlink = input.downlink_per_day.map(Qty::value);
    checks.extend(summary_checks(
        p.reserve, share_sum, margin, &parts, downlink,
    )?);
    Ok(DataBudgetOutput {
        hk_per_day: Qty::from_raw(hk),
        mission_per_day: Qty::from_raw(mission),
        total_per_day: Qty::from_raw(total),
        average_rate: Qty::from_raw(total / SECONDS_PER_DAY),
        modes,
        parts,
        storage_capacity: Qty::from_raw(p.storage_bits),
        storage_reserve: p.reserve,
        required_capacity_margin: required_capacity_margin(p.reserve),
        storage_required: Qty::from_raw(required),
        capacity_margin: margin,
        downlink_balance_per_day: balance.map(Qty::from_raw),
        checks,
    })
}

/// モードの割合・ストレージ余裕・送信の収支の判定。収支は `budget::limit_check`（部品ごとの 1 日の発生量 ≤ 送信可能量）。
fn summary_checks(
    reserve: Fraction,
    share_sum: f64,
    margin: Option<Fraction>,
    parts: &[DataBudgetPartResult],
    downlink_per_day: Option<f64>,
) -> Result<Vec<Check>, AnalysisError> {
    let mut checks = Vec::new();
    if share_sum < 1.0 - DUTY_TOLERANCE {
        checks.push(Check::new(
            "mode_shares",
            "モードの割合",
            Verdict::Warning,
            format!(
                "モードの割合の合計が {}（残りはデータを出さない時間とみなします）",
                quantity_text::<Fraction>(share_sum)
            ),
        ));
    }
    // 警告の幅 = 容量の余裕の要求値（残す割合から）。必要量が容量の (1 − 残す割合) 倍を超えると警告、容量を超えると違反
    let required = required_capacity_margin(reserve);
    let shown = margin.map_or("必要量 0".to_owned(), |m| {
        quantity_text::<Fraction>(m.value())
    });
    checks.push(Check::new(
        "storage_margin",
        "ストレージ余裕",
        ratio_margin_verdict(margin, required),
        format!(
            "容量の余裕 {shown}（要求 {} 以上 = 容量の {} を残す）",
            quantity_text::<Fraction>(required.value()),
            quantity_text::<Fraction>(reserve.value())
        ),
    ));
    if let Some(limit) = downlink_per_day {
        let items: Vec<BudgetItem> = parts
            .iter()
            .map(|p| {
                let per_day = p.hk_per_day.value() + p.mission_per_day.value();
                BudgetItem::counted(p.part.clone(), per_day, 1)
            })
            .collect();
        checks.extend(limit_check::<Information>(
            &items,
            limit,
            "downlink",
            "1 日の送信可能量と発生量",
        )?);
    }
    Ok(checks)
}

#[cfg(test)]
mod tests;
