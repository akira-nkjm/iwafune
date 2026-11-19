use super::*;

fn range(min: f64, max: f64) -> ThermalTempRange {
    ThermalTempRange {
        min: Qty::from_raw(min),
        max: Qty::from_raw(max),
    }
}

/// モードによらない節点の範囲（`ThermalNode::ranges_in` に空のモードを渡したときと同じ）。
fn all(r: &ThermalRanges) -> Vec<ThermalNodeRange> {
    r.iter()
        .map(|(kind, range)| ThermalNodeRange {
            kind,
            range,
            excluded: Vec::new(),
        })
        .collect()
}

fn verdicts(lo: f64, hi: f64, ranges: &ThermalRanges, margin: ThermalMarginPolicy) -> Vec<Verdict> {
    range_margins(lo, hi, &all(ranges), &margin.used())
        .iter()
        .map(|m| m.verdict)
        .collect()
}

/// 動作範囲 270〜320 K: 280〜300 K は端から 10 K 離れて Ok、316 K は上端まで 4 K（< 5 K）で警告、
/// 321 K は超過。範囲が無ければ比較も判定文も無く、全体の判定は Ok。
#[test]
fn verdict_by_distance_to_the_ends() {
    let r = ThermalRanges {
        operating: Some(range(270.0, 320.0)),
        ..ThermalRanges::default()
    };
    let p = ThermalMarginPolicy::default();
    assert_eq!(verdicts(280.0, 300.0, &r, p), [Verdict::Ok]);
    assert_eq!(verdicts(280.0, 316.0, &r, p), [Verdict::Warning]);
    assert_eq!(verdicts(280.0, 321.0, &r, p), [Verdict::Violation]);
    assert_eq!(verdicts(268.0, 300.0, &r, p), [Verdict::Violation]);
    let none = range_margins(0.0, 1.0e4, &all(&ThermalRanges::default()), &p.used());
    assert!(none.is_empty());
    assert_eq!(worst_verdict(&none), Verdict::Ok);
    let m = range_margins(280.0, 316.0, &all(&r), &p.used());
    let checks = range_checks("box", &m, &p.used());
    assert_eq!(checks[0].key, "range_high:box:operating");
    assert_eq!(checks[1].key, "range_low:box:operating");
    // 316 K は上限まで 4 K（警告）、下限まで 10 K（OK）: 判定は側ごとに分かれる
    assert_eq!(checks[0].verdict, Verdict::Warning);
    assert_eq!(checks[1].verdict, Verdict::Ok);
    assert!((m[0].margin_low.value() - 10.0).abs() < 1e-12);
    assert!((m[0].margin_high.value() - 4.0).abs() < 1e-12);
}

/// 保管 250〜340 K・動作 270〜320 K・性能 285〜305 K に 280〜300 K を当てる: 保管は端から 30 K（Ok）、
/// 動作は 10 K（Ok）、性能は下端を 5 K 下回る（違反）。不確かさ 8 K を足すと動作は 2 K（< 5 K）で警告、
/// 保管は 22 K で Ok のまま。警告の幅を 25 K にすると保管も警告。
#[test]
fn three_ranges_and_uncertainty_by_hand() {
    let r = ThermalRanges {
        storage: Some(range(250.0, 340.0)),
        operating: Some(range(270.0, 320.0)),
        performance: Some(range(285.0, 305.0)),
    };
    use Verdict::{Ok, Violation, Warning};
    let none = ThermalMarginPolicy::default();
    assert_eq!(verdicts(280.0, 300.0, &r, none), [Ok, Ok, Violation]);
    let u8 = ThermalMarginPolicy {
        uncertainty: Some(Qty::from_raw(8.0)),
        warning_band: None,
        heater_duty_warning: None,
    };
    assert_eq!(verdicts(280.0, 300.0, &r, u8), [Ok, Warning, Violation]);
    let wide = ThermalMarginPolicy {
        warning_band: Some(Qty::from_raw(25.0)),
        ..u8
    };
    assert_eq!(
        verdicts(280.0, 300.0, &r, wide),
        [Warning, Warning, Violation]
    );
    let m = range_margins(280.0, 300.0, &all(&r), &u8.used());
    let keys: Vec<String> = range_checks("n", &m, &u8.used())
        .into_iter()
        .map(|c| c.key)
        .collect();
    assert_eq!(
        keys,
        [
            "range_high:n:storage",
            "range_low:n:storage",
            "range_high:n:operating",
            "range_low:n:operating",
            "range_high:n:performance",
            "range_low:n:performance"
        ]
    );
    assert_eq!(worst_verdict(&m), Violation);
}

/// 種類ごとの共通部分: 片方にしか無い種類はその範囲のまま。
#[test]
fn ranges_intersect_per_kind() {
    let a = ThermalRanges {
        operating: Some(range(233.0, 343.0)),
        storage: Some(range(223.0, 353.0)),
        performance: None,
    };
    let b = ThermalRanges {
        operating: Some(range(253.0, 358.0)),
        storage: None,
        performance: Some(range(273.0, 303.0)),
    };
    let c = a.intersect(b);
    assert_eq!(c.operating, Some(range(253.0, 343.0)));
    assert_eq!(c.storage, a.storage);
    assert_eq!(c.performance, b.performance);
}

/// 全ケースのまとめ（手計算）: 動作の範囲がケースで違う（低温ケースは 260〜320 K で 265 K、高温ケースは 270〜310 K で
/// 300 K）。下限の側は低温ケースの 5 K、上限の側は高温ケースの 10 K。共通部分 270〜310 K と最低 265 K で比べると
/// 違反になるが、ケースごとには違反ではない（警告の幅 5 K なので下限ちょうど 5 K は Ok）。保管は低温ケースだけにあり、
/// そのケースの値。範囲がケースで同じなら、全ケースの最低・最高と範囲を比べるのと同じ。
#[test]
fn envelope_uses_each_case_range_by_hand() {
    let p = ThermalMarginPolicy::default().used();
    let node = |op: (f64, f64), storage: Option<(f64, f64)>| ThermalRanges {
        operating: Some(range(op.0, op.1)),
        storage: storage.map(|(a, b)| range(a, b)),
        performance: None,
    };
    let cold = range_margins(
        265.0,
        265.0,
        &all(&node((260.0, 320.0), Some((240.0, 340.0)))),
        &p,
    );
    let hot = range_margins(300.0, 300.0, &all(&node((270.0, 310.0), None)), &p);
    let env = envelope_margins([cold.as_slice(), hot.as_slice()].into_iter(), &p);
    assert_eq!(env.len(), 2);
    assert_eq!(env[0].kind, ThermalRangeKind::Storage);
    assert_eq!(env[0].margin_low.value(), 25.0);
    let op = &env[1];
    assert_eq!(op.range, range(260.0, 310.0));
    assert_eq!((op.margin_low.value(), op.margin_high.value()), (5.0, 10.0));
    assert_eq!(op.verdict, Verdict::Ok);
    // 判定文の予測の温度は余裕から戻した値（下限の側 265 K、上限の側 300 K）
    let back = |m: &ThermalRangeMargin| m.predicted();
    assert_eq!(back(op), (265.0, 300.0));
    // 範囲が同じなら全ケースの最低・最高（265〜300 K）で比べるのと同じ
    let same = all(&node((260.0, 320.0), None));
    let a = range_margins(265.0, 265.0, &same, &p);
    let b = range_margins(300.0, 300.0, &same, &p);
    let env = envelope_margins([a.as_slice(), b.as_slice()].into_iter(), &p);
    let whole = range_margins(265.0, 300.0, &same, &p);
    assert_eq!(env[0].margin_low.value(), whole[0].margin_low.value());
    assert_eq!(env[0].margin_high.value(), whole[0].margin_high.value());
    assert_eq!(env[0].verdict, whole[0].verdict);
}

/// 外した部品の文: 同じ部品の組を外したモードは 1 つにまとめる。
#[test]
fn excluded_parts_text_groups_modes() {
    let e = |part: &str, mode: &str, power| ThermalRangeExclusion {
        part: part.into(),
        mode: mode.into(),
        power_state: power,
    };
    use ThermalPowerState::{Off, On};
    let text = excluded_text(&[
        e("STX", "NOMINAL", Off),
        e("STX", "MISSION", Off),
        e("GPS", "SAFE", Off),
        e("STX", "SAFE", Off),
        e("OBC", "SAFE", On),
    ]);
    assert_eq!(
        text,
        ". Excludes powered-off STX in mode NOMINAL, MISSION. Excludes powered-off GPS, STX in mode SAFE. Excludes powered-on OBC in mode SAFE"
    );
    assert!(excluded_text(&[]).is_empty());
}

/// 側ごとの判定: 動作範囲 270〜320 K に 280〜321 K（上限だけ 1 K 超過）を当てると、上限側は違反、下限側は
/// 下限まで 10 K で OK。逆に 268〜300 K なら下限側だけ違反。不確かさ 8 K では下限側（280 K − 8 K → 余裕 2 K）だけ警告。
#[test]
fn each_side_is_judged_on_its_own() {
    let r = ThermalRanges {
        operating: Some(range(270.0, 320.0)),
        ..ThermalRanges::default()
    };
    let sides = |lo: f64, hi: f64, p: ThermalMarginPolicy| -> (Verdict, Verdict) {
        let used = p.used();
        let c = range_checks("n", &range_margins(lo, hi, &all(&r), &used), &used);
        (c[0].verdict, c[1].verdict)
    };
    let p = ThermalMarginPolicy::default();
    assert_eq!(sides(280.0, 321.0, p), (Verdict::Violation, Verdict::Ok));
    assert_eq!(sides(268.0, 300.0, p), (Verdict::Ok, Verdict::Violation));
    let u8 = ThermalMarginPolicy {
        uncertainty: Some(Qty::from_raw(8.0)),
        warning_band: None,
        heater_duty_warning: None,
    };
    assert_eq!(sides(280.0, 300.0, u8), (Verdict::Ok, Verdict::Warning));
    // 範囲全体の判定は両側の悪いほう
    let m = range_margins(280.0, 321.0, &all(&r), &p.used());
    assert_eq!(m[0].verdict, Verdict::Violation);
}

/// 判定の key から側を除いた項目が取れる（上限側と下限側は同じ項目。温度範囲でない key は None）。
#[test]
fn range_item_drops_the_side() {
    assert_eq!(
        range_item("range_high:box:operating"),
        Some("box:operating")
    );
    assert_eq!(range_item("range_low:box:operating"), Some("box:operating"));
    assert_eq!(range_item("range_high"), None);
    assert_eq!(range_item("range_highest:x:y"), None);
    assert_eq!(range_item("dod"), None);
}
