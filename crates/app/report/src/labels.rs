//! 表・図の見出しと注記の日本語・英語の対。**書き出す文言はここだけ**（設計データの名前・解析の表示名は service が渡す）。
//! 文言の `{}` は引数で順に置き換える（[`tr`]）。

use crate::model::Lang;

/// 文言の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    // 共通
    Item,
    Quantity,
    Parameter,
    Value,
    Unit,
    Node,
    Component,
    Minimum,
    Maximum,
    Verdict,
    VerdictOk,
    VerdictWarning,
    VerdictViolation,
    Uplink,
    Downlink,
    Other,
    CaseDefault,
    NotAvailable,
    ModelRevision,
    SignificantFigures,
    // 質量表
    MassCaption,
    UnitMass,
    Mass,
    Growth,
    MassWithGrowth,
    Subtotal,
    TotalDry,
    Limit,
    SystemMargin,
    Margin,
    Propellant,
    TotalWet,
    MassNote,
    // モードごとの電力表
    PowerCaption,
    TotalWithoutMargin,
    TotalWithMargin,
    EclipseWithoutMargin,
    EclipseWithMargin,
    PeakWithMargin,
    PowerNote,
    // 回線バジェット
    LinkInputCaption,
    LinkResultCaption,
    Frequency,
    TxPower,
    TxGain,
    TxLoss,
    RxGain,
    RxLoss,
    NoiseTemperature,
    RxGOverT,
    AtmosphericLoss,
    PolarizationLoss,
    PointingLoss,
    OtherLoss,
    DataRate,
    RequiredEbN0,
    Altitude,
    Elevation,
    SlantRange,
    RangeMin,
    LightTime,
    Eirp,
    Fspl,
    ExtraLoss,
    GOverT,
    CN0,
    EbN0,
    LinkMargin,
    MaxDataRate,
    Pfd,
    PfdLimit,
    PfdMargin,
    RequiredMargin,
    LinkNote,
    // 温度
    ThermalCaption,
    AllowableRange,
    ThermalNote,
    ThermalHistoryCaption,
    PowerHistoryCaption,
    Time,
    Temperature,
    Power,
    StateOfCharge,
    Generation,
    Load,
    ChargePower,
    DischargePower,
    Eclipse,
    AllowableLegend,
    RangeStorage,
    RangeOperating,
    RangePerformance,
    ValueMassTotal,
    ValueMassLimit,
    ValueMassMargin,
}

/// （日本語, 英語）。
fn pair(t: Text) -> (&'static str, &'static str) {
    use Text::*;
    match t {
        Item => ("項目", "Item"),
        Quantity => ("数量", "Qty"),
        Parameter => ("項目", "Parameter"),
        Value => ("値", "Value"),
        Unit => ("単位", "Unit"),
        Node => ("節点", "Node"),
        Component => ("機器", "Component"),
        Minimum => ("最低", "Min"),
        Maximum => ("最高", "Max"),
        Verdict => ("判定", "Verdict"),
        VerdictOk => ("適合", "OK"),
        VerdictWarning => ("注意", "Warning"),
        VerdictViolation => ("不適合", "Violation"),
        Uplink => ("アップリンク", "uplink"),
        Downlink => ("ダウンリンク", "downlink"),
        Other => ("その他", "Other"),
        CaseDefault => ("既定のケース", "default case"),
        NotAvailable => ("算出できません: {}", "Not available: {}"),
        ModelRevision => ("モデルの revision: {}", "Model revision: {}"),
        SignificantFigures => (
            "数値は有効数字 {} 桁。",
            "Values are given to {} significant figures.",
        ),
        MassCaption => ("質量バジェット", "Mass budget"),
        UnitMass => ("単体質量", "Unit mass"),
        Mass => ("質量", "Mass"),
        Growth => ("成長余裕", "Growth"),
        MassWithGrowth => ("余裕込み質量", "Mass with growth"),
        Subtotal => ("{} 小計", "{} subtotal"),
        TotalDry => (
            "合計（乾燥・成長余裕込み）",
            "Total (dry, with growth margin)",
        ),
        Limit => ("上限", "Limit"),
        SystemMargin => (
            "システムマージン（上限に対する割合 {}）",
            "System margin ({} of the limit)",
        ),
        Margin => ("余裕", "Margin"),
        Propellant => ("推薬", "Propellant"),
        TotalWet => ("合計（推薬込み）", "Total (wet)"),
        MassNote => (
            "質量は成長余裕（成熟度ごと）を含む。余裕は上限からシステムマージンを引いた量との差。推薬には成長余裕を掛けない。",
            "Masses include the growth margin set per maturity. The margin is measured against the limit less the system margin. No growth margin is applied to propellant.",
        ),
        PowerCaption => ("モードごとの消費電力", "Power consumption by mode"),
        TotalWithoutMargin => ("合計（余裕なし）", "Total (without margin)"),
        TotalWithMargin => ("合計（成長余裕込み）", "Total (with growth margin)"),
        EclipseWithoutMargin => ("蝕中の合計（余裕なし）", "Eclipse total (without margin)"),
        EclipseWithMargin => (
            "蝕中の合計（成長余裕込み）",
            "Eclipse total (with growth margin)",
        ),
        PeakWithMargin => ("ピーク（成長余裕込み）", "Peak (with growth margin)"),
        PowerNote => (
            "機器の欄は成長余裕を含まない平均消費電力（括弧内は状態と割合）。合計は余裕なしと成長余裕込みの両方を示す。",
            "Component cells are mean power without growth margin (state and time fraction in brackets). Totals are given both without and with the growth margin.",
        ),
        LinkInputCaption => ("回線バジェットの入力: {}", "Link budget inputs: {}"),
        LinkResultCaption => ("回線バジェットの結果: {}", "Link budget results: {}"),
        Frequency => ("周波数", "Frequency"),
        TxPower => ("送信電力", "Tx power"),
        TxGain => ("送信アンテナ利得", "Tx antenna gain"),
        TxLoss => ("送信側の損失", "Tx loss"),
        RxGain => ("受信アンテナ利得", "Rx antenna gain"),
        RxLoss => ("受信側の損失", "Rx loss"),
        NoiseTemperature => ("システム雑音温度", "System noise temperature"),
        RxGOverT => ("受信 G/T（与えられた値）", "Rx G/T (given)"),
        AtmosphericLoss => ("大気損失", "Atmospheric loss"),
        PolarizationLoss => ("偏波損失", "Polarization loss"),
        PointingLoss => ("指向損失", "Pointing loss"),
        OtherLoss => ("その他の損失", "Other loss"),
        DataRate => ("データレート", "Data rate"),
        RequiredEbN0 => ("所要 Eb/N0", "Required Eb/N0"),
        Altitude => ("高度", "Altitude"),
        Elevation => ("仰角", "Elevation"),
        SlantRange => ("斜距離", "Slant range"),
        RangeMin => ("最小の距離", "Minimum distance"),
        LightTime => (
            "光の片道時間（最大の距離）",
            "One-way light time (maximum distance)",
        ),
        Eirp => ("EIRP", "EIRP"),
        Fspl => ("自由空間損失", "Free-space loss"),
        ExtraLoss => ("そのほかの経路損失", "Other path losses"),
        GOverT => ("受信 G/T", "Rx G/T"),
        CN0 => ("C/N0", "C/N0"),
        EbN0 => ("Eb/N0", "Eb/N0"),
        LinkMargin => ("回線余裕", "Link margin"),
        MaxDataRate => (
            "要求余裕で通せる最大データレート",
            "Maximum data rate at the required margin",
        ),
        Pfd => (
            "地表の電力束密度（PFD）",
            "Power flux density at the ground (PFD)",
        ),
        PfdLimit => ("PFD の限度", "PFD limit"),
        PfdMargin => ("PFD の余裕", "PFD margin"),
        RequiredMargin => ("要求余裕", "Required margin"),
        LinkNote => (
            "回線余裕 = Eb/N0 − 所要 Eb/N0。判定は要求余裕に対して行う。",
            "Link margin = Eb/N0 minus required Eb/N0. The verdict is against the required margin.",
        ),
        ThermalCaption => ("温度解析の結果（{}）", "Thermal analysis results ({})"),
        AllowableRange => ("許容範囲（{}）", "Allowable range ({})"),
        ThermalNote => (
            "ケースの列は定常解析の節点温度。最低・最高はケース全体の範囲。判定には不確かさ {} と注意の帯 {} を含む。",
            "Case columns are steady-state node temperatures. Min and max span all cases. Verdicts include an uncertainty of {} and a warning band of {}.",
        ),
        ThermalHistoryCaption => (
            "節点温度の時間変化（{}）",
            "Node temperatures over time ({})",
        ),
        PowerHistoryCaption => (
            "発電・消費電力と電池の充電状態の時間変化（{}）",
            "Generation, load and battery state of charge over time ({})",
        ),
        Time => ("経過時間", "Elapsed time"),
        Temperature => ("温度", "Temperature"),
        Power => ("電力", "Power"),
        StateOfCharge => ("充電状態", "State of charge"),
        Generation => ("発電", "Generation"),
        Load => ("消費", "Load"),
        ChargePower => ("充電", "Charge"),
        DischargePower => ("放電", "Discharge"),
        Eclipse => ("蝕", "Eclipse"),
        AllowableLegend => ("許容範囲（{}）: {}", "Allowable range ({}): {}"),
        RangeStorage => ("保管", "storage"),
        RangeOperating => ("動作", "operating"),
        RangePerformance => ("性能", "performance"),
        ValueMassTotal => (
            "質量の合計（乾燥・成長余裕込み）",
            "Total dry mass with growth margin",
        ),
        ValueMassLimit => ("質量の上限", "Mass limit"),
        ValueMassMargin => ("質量の余裕", "Mass margin"),
    }
}

/// 言語に合う固定の文言。
pub fn t(lang: Lang, text: Text) -> &'static str {
    let (ja, en) = pair(text);
    match lang {
        Lang::Ja => ja,
        Lang::En => en,
    }
}

/// `{}` を引数で順に置き換えた文言。
pub fn tr(lang: Lang, text: Text, args: &[&str]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut rest = t(lang, text);
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        out.push_str(args.next().copied().unwrap_or(""));
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled_in_order() {
        assert_eq!(
            tr(Lang::En, Text::AllowableLegend, &["operating", "BAT"]),
            "Allowable range (operating): BAT"
        );
        assert_eq!(t(Lang::Ja, Text::Item), "項目");
    }
}
