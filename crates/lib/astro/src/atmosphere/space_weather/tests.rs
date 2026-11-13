use super::*;

const HEADER: &str = "DATE,BSRN,ND,KP1,KP2,KP3,KP4,KP5,KP6,KP7,KP8,KP_SUM,AP1,AP2,AP3,AP4,AP5,AP6,AP7,AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,F10.7_OBS_CENTER81,F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81";

/// 2003 年 10 月下旬（ハロウィーンの磁気嵐）の 7 日分。CelesTrak の SW-All（2026-11-13 に取得）の行そのまま。
///
/// 3 時間 ap・日平均 Ap・F10.7（観測・1 AU 補正）は、GFZ Potsdam が公開している確定値の表
/// （`Kp_ap_Ap_SN_F107_since_1932.txt`）の同じ日の値とも一致する（10-29 は Ap = 204、F10.7 = 291.7）。
const HALLOWEEN_2003: &str = "\
2003-10-26,2323,24,23,20,17,20,10,10,33,37,170,9,7,6,7,4,4,18,22,10,0.5,2,133,298.3,294.7,OBS,147.0,122.3,145.2,123.4
2003-10-27,2323,25,37,33,33,20,20,23,13,10,190,22,18,18,7,7,9,5,4,11,0.6,3,199,257.2,254.0,OBS,147.1,123.8,145.3,124.8
2003-10-28,2323,26,30,47,37,47,27,40,33,40,300,15,39,22,39,12,27,18,27,25,1.2,6,247,274.4,270.9,OBS,147.0,125.6,145.1,126.5
2003-10-29,2323,27,47,40,90,80,77,77,87,87,583,39,27,400,207,179,179,300,300,204,2.1,9,250,291.7,287.7,OBS,146.8,127.6,144.8,128.4
2003-10-30,2324,1,87,73,53,47,50,70,90,90,560,300,154,56,39,48,132,400,400,191,2.1,9,250,271.4,267.6,OBS,146.5,129.3,144.6,130.0
2003-10-31,2324,2,83,77,73,67,73,47,40,43,503,236,179,154,111,154,39,27,32,116,1.9,8,239,248.9,245.2,OBS,146.2,130.8,144.2,131.4
2003-11-01,2324,3,50,47,40,27,30,33,33,40,300,48,39,27,12,15,18,18,27,26,1.2,6,153,210.4,207.2,OBS,145.8,131.8,143.8,132.4
";

/// 実績値の後に予測が続く部分の抜き書き（同じ 2026-11-13 に取得した SW-All の行。間の行は省いた）。
/// 日ごとの予測（PRD）は 3 時間の値も入っていて、月ごとの予測（PRM）は月の 1 日に 1 行で Kp・Ap が空欄。
/// 日ごとの予測の最後（12-27）と月ごとの予測の最初（2027-01-01）の間に行の無い日がある。
const FORECAST_2026: &str = "\
2026-11-11,2635,17,10,7,13,23,3,13,27,13,110,4,3,5,9,2,5,12,5,6,0.3,1,121,131.9,129.3,OBS,106.9,110.8,105.0,110.9
2026-11-12,2635,18,13,17,13,3,0,3,13,17,80,5,6,5,2,0,2,5,6,4,0.1,0,107,115.5,113.2,OBS,106.8,110.6,104.9,110.7
2026-11-13,2635,19,10,0,10,7,3,13,13,13,70,4,0,4,3,2,5,5,5,3,0.1,0,139,117.4,115.0,PRD,106.8,110.3,104.8,110.3
2026-11-14,2635,20,13,13,13,13,13,13,13,13,104,5,5,5,5,5,5,5,5,5,0.2,1,74,112.3,110.0,PRD,106.8,110.0,104.8,110.0
2026-12-26,2637,8,24,24,24,24,24,24,24,24,192,10,10,10,10,10,10,10,10,10,0.6,3,72,103.4,100.0,PRD,108.4,106.7,105.1,104.6
2026-12-27,2637,9,13,13,13,13,13,13,13,13,104,5,5,5,5,5,5,5,5,5,0.2,1,72,108.6,105.0,PRD,108.5,106.6,105.2,104.5
2027-01-01,2637,14,,,,,,,,,,,,,,,,,,,,,70,117.5,113.6,PRM,109.6,106.4,106.3,104.1
2027-02-01,2638,18,,,,,,,,,,,,,,,,,,,,,68,115.1,111.7,PRM,114.4,108.3,111.4,105.0
";

fn table(rows: &str) -> SpaceWeatherTable {
    SpaceWeatherTable::from_csv_bytes(format!("{HEADER}\r\n{rows}").as_bytes()).expect("表")
}

fn at(s: &str) -> Epoch {
    Epoch::parse(s).expect("元期")
}

#[test]
fn reads_the_published_columns() {
    let t = table(HALLOWEEN_2003);
    assert_eq!(t.days().len(), 7);
    let d = t.day(Epoch::day_number_of(2003, 10, 29)).expect("10-29");
    assert_eq!(d.qualifier, FluxQualifier::Observed);
    assert_eq!(d.ap_daily, Some(204.0));
    assert_eq!(
        d.ap_3h,
        Some([39.0, 27.0, 400.0, 207.0, 179.0, 179.0, 300.0, 300.0])
    );
    assert_eq!(d.f107_obs, Some(291.7));
    assert_eq!(d.f107_obs_center81, Some(146.8));
    assert_eq!(d.f107_obs_last81, Some(127.6));
    let c = t.coverage();
    assert_eq!(c.first, Epoch::day_number_of(2003, 10, 26));
    assert_eq!(c.last_observed, Some(Epoch::day_number_of(2003, 11, 1)));
    assert_eq!(c.last, c.last_observed.expect("実績"));
}

/// 2003-10-30 07:30 UT: F10.7 は前日（10-29）の 291.7、81 日平均は当日の中心平均、Ap は当日の 191。
/// 3 時間の履歴は 06〜09 UT の 56 から遡る（−12〜−33 h は 10-29 の 7 区間と 10-28 の最後の区間の平均）。
#[test]
fn looks_up_observed_values_by_date() {
    let t = table(HALLOWEEN_2003);
    let w = space_weather_at(Some(&t), at("2003-10-30T07:30:00Z"), SolarActivity::Low);
    assert_eq!((w.f107, w.f107a, w.ap), (291.7, 146.5, 191.0));
    assert_eq!(w.f107_source, WeatherSource::Observed);
    assert_eq!(w.ap_source, WeatherSource::Observed);
    let mid = (300.0 + 179.0 + 179.0 + 207.0 + 400.0 + 27.0 + 39.0 + 27.0) / 8.0;
    let old = (18.0 + 27.0 + 12.0 + 39.0 + 22.0 + 39.0 + 15.0 + 4.0) / 8.0;
    assert_eq!(
        w.ap_history,
        Some([191.0, 56.0, 154.0, 300.0, 300.0, mid, old])
    );
    assert_eq!(
        w.nrlmsise00().geomagnetic,
        GeomagneticActivity::ThreeHourlyAp([191.0, 56.0, 154.0, 300.0, 300.0, mid, old])
    );
    assert_eq!(
        w.atmosphere_model(),
        AtmosphereModel::Nrlmsise00 {
            f107: 291.7,
            f107a: 146.5,
            ap: 191.0
        }
    );
}

#[test]
fn incomplete_history_keeps_the_daily_ap() {
    // 10-27 は 57 時間前の区間が表の外（10-25 が無い）
    let t = table(HALLOWEEN_2003);
    let w = space_weather_at(Some(&t), at("2003-10-27T03:00:00Z"), SolarActivity::Low);
    assert_eq!((w.f107, w.ap, w.ap_history), (298.3, 11.0, None));
    assert_eq!(
        w.nrlmsise00().geomagnetic,
        GeomagneticActivity::DailyAp(11.0)
    );
}

#[test]
fn outside_the_table_uses_the_preset() {
    let t = table(HALLOWEEN_2003);
    for e in ["2003-10-26T12:00:00Z", "2003-11-03T00:00:00Z", "1990-01-01"] {
        let w = space_weather_at(Some(&t), at(e), SolarActivity::High);
        assert_eq!(w.f107_source, WeatherSource::Preset, "{e}");
        assert_eq!((w.f107, w.f107a), (250.0, 250.0), "{e}");
    }
    // 10-26 は前日が無いので F10.7 だけプリセット、Ap は表の値
    let w = space_weather_at(Some(&t), at("2003-10-26T12:00:00Z"), SolarActivity::High);
    assert_eq!((w.ap, w.ap_source), (10.0, WeatherSource::Observed));
    assert_eq!(
        space_weather_at(None, at("2003-10-29"), SolarActivity::Moderate),
        DatedSpaceWeather::preset(SolarActivity::Moderate)
    );
}

#[test]
fn presets_follow_ecss_table_6_3() {
    assert_eq!(SolarActivity::Low.indices(), (65.0, 65.0, 0.0));
    assert_eq!(SolarActivity::Moderate.indices(), (140.0, 140.0, 15.0));
    assert_eq!(SolarActivity::High.indices(), (250.0, 250.0, 45.0));
    assert_eq!(SolarActivity::default(), SolarActivity::Moderate);
    let m = DatedSpaceWeather::manual(100.0, 110.0, 7.0);
    assert_eq!(m.f107_source, WeatherSource::Manual);
    assert_eq!(
        m.nrlmsise00().geomagnetic,
        GeomagneticActivity::DailyAp(7.0)
    );
}

#[test]
fn predicted_rows_and_monthly_predictions() {
    let t = table(FORECAST_2026);
    let c = t.coverage();
    assert_eq!(c.last_observed, Some(Epoch::day_number_of(2026, 11, 12)));
    assert_eq!(c.last, Epoch::day_number_of(2027, 2, 28));
    // 日ごとの予測の初日: 前日は実績、当日は予測なので F10.7 は予測を名乗る。3 時間の履歴も揃う
    let w = space_weather_at(Some(&t), at("2026-11-13T12:00:00Z"), SolarActivity::Low);
    assert_eq!((w.f107, w.f107a, w.ap), (115.5, 106.8, 3.0));
    assert_eq!(w.f107_source, WeatherSource::DailyPredicted);
    assert_eq!(w.ap_source, WeatherSource::DailyPredicted);
    assert_eq!(w.ap_history.map(|h| h[1]), Some(2.0));
    // 日ごとと月ごとの予測の間の日は直前の行を保つ（3 時間の履歴は作らない）
    let w = space_weather_at(Some(&t), at("2026-12-30T00:00:00Z"), SolarActivity::Low);
    assert_eq!((w.f107, w.f107a, w.ap), (108.6, 108.5, 5.0));
    assert_eq!(w.f107_source, WeatherSource::DailyPredicted);
    assert_eq!(w.ap_history, None);
    // 月ごとの予測はその月のすべての日に当てる。Ap が空欄なのでプリセット
    let w = space_weather_at(Some(&t), at("2027-01-15T00:00:00Z"), SolarActivity::Low);
    assert_eq!((w.f107, w.f107a), (117.5, 109.6));
    assert_eq!(w.f107_source, WeatherSource::MonthlyPredicted);
    assert_eq!((w.ap, w.ap_source), (0.0, WeatherSource::Preset));
    // 2 月 1 日の前日は 1 月の行
    let w = space_weather_at(Some(&t), at("2027-02-01T00:00:00Z"), SolarActivity::Low);
    assert_eq!((w.f107, w.f107a), (117.5, 114.4));
    // 最後の月の末日までは使い、その後はプリセット
    let w = space_weather_at(Some(&t), at("2027-02-28T00:00:00Z"), SolarActivity::Low);
    assert_eq!(w.f107_source, WeatherSource::MonthlyPredicted);
    let w = space_weather_at(Some(&t), at("2027-03-01T00:00:00Z"), SolarActivity::Low);
    assert_eq!(w.f107_source, WeatherSource::Preset);
}

#[test]
fn rejects_what_is_not_the_table() {
    let html = b"<!DOCTYPE HTML>\n<HTML><BODY>Service Unavailable</BODY></HTML>";
    assert_eq!(
        SpaceWeatherTable::from_csv_bytes(html),
        Err(SpaceWeatherParseError::MissingColumn("DATE"))
    );
    assert_eq!(
        SpaceWeatherTable::from_csv_bytes(HEADER.as_bytes()),
        Err(SpaceWeatherParseError::Empty)
    );
    let bad = format!("{HEADER}\n2003-10-27,,,,,,,,,,,,,,,,,,,,1,,,,1.0,,XXX,1.0,,,");
    assert!(matches!(
        SpaceWeatherTable::from_csv_bytes(bad.as_bytes()),
        Err(SpaceWeatherParseError::Row { line: 2, .. })
    ));
    let unordered = format!(
        "{HEADER}\n{}",
        HALLOWEEN_2003.lines().rev().collect::<Vec<_>>().join("\n")
    );
    assert!(matches!(
        SpaceWeatherTable::from_csv_bytes(unordered.as_bytes()),
        Err(SpaceWeatherParseError::Row { line: 3, .. })
    ));
}
