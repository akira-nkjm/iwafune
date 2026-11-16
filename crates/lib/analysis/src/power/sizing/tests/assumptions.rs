//! 仮定（既定の効率・設計寿命・電池の値の不一致）が出力の `used` と `checks` に出ること。
use super::*;

#[test]
fn default_path_efficiencies_are_the_named_constants() {
    let o = Sizing::run(&input()).unwrap();
    assert_eq!(o.used.sunlit_efficiency, 0.8);
    assert_eq!(o.used.eclipse_efficiency, 0.6);
    assert_eq!(DEFAULT_SUNLIT_EFFICIENCY, 0.8);
    assert_eq!(DEFAULT_ECLIPSE_EFFICIENCY, 0.6);
}

/// 寿命は設計寿命の入力があればそれ、無ければ季節の走査の期間。
/// 年あたり 3 % の劣化: 走査 1 日（= 1/365.25 年）なら 0.97^(1/365.25)、設計寿命 10 年なら 0.97^10 = 0.737424…。
/// 必要面積は劣化に反比例するので、面積の比は劣化の比。
#[test]
fn design_life_replaces_the_season_span_when_given() {
    let mut i = input();
    i.options.annual_degradation = Some(0.03);
    let span = Sizing::run(&i).unwrap();
    assert_eq!(span.used.lifetime_source, SizingLifetimeSource::SeasonSpan);
    let days = 1.0 / 365.25;
    assert!((span.used.lifetime_years - days).abs() < 1e-12);
    assert!((span.used.life_factor - 0.97_f64.powf(days)).abs() < 1e-12);

    i.design_life = Some(Qty::from_raw(10.0 * 365.25 * 86400.0));
    let life = Sizing::run(&i).unwrap();
    assert_eq!(life.used.lifetime_source, SizingLifetimeSource::DesignLife);
    assert!((life.used.lifetime_years - 10.0).abs() < 1e-12);
    let hand = 0.97_f64.powi(10); // 0.737424…
    assert!((life.used.life_factor - hand).abs() < 1e-12);
    let ratio = life.smad.required_area.value() / span.smad.required_area.value();
    assert!(
        (ratio - span.used.life_factor / hand).abs() < 1e-9,
        "{ratio}"
    );
    assert!(ratio > 1.0);
}

#[test]
fn unequal_batteries_add_one_warning_to_sizing() {
    let mut i = input();
    assert!(
        Sizing::run(&i)
            .unwrap()
            .checks
            .iter()
            .all(|c| c.key != "battery_mismatch")
    );
    let mut other = i.batteries[0].clone();
    other.voltage = Qty::from_raw(14.0);
    i.batteries.push(other);
    let o = Sizing::run(&i).unwrap();
    let found: Vec<_> = o
        .checks
        .iter()
        .filter(|c| c.key == "battery_mismatch")
        .collect();
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("Nominal voltage"));
    // 同じ 2 つなら注意なし、容量は 2 倍
    let mut same = input();
    same.batteries.push(same.batteries[0].clone());
    let o = Sizing::run(&same).unwrap();
    assert!(o.checks.iter().all(|c| c.key != "battery_mismatch"));
    assert_eq!(o.design_capacity.value(), 720_000.0);
}
