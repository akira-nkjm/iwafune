use super::*;
use crate::{NAMED_UNIT_SYMBOLS, UnitError, parse_unit};

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-12 * right.abs().max(left.abs())
}

/// 接頭辞の追加・制限の前から読めていた綴りが、同じ SI の倍率・次元のまま読める。
///
/// 期待値は接頭辞の表から導かずに、変更前の実行結果を書き写した定数（倍率）と、
/// 接頭辞を含まない SI の式（次元）で持つ。デモの設計データ・文書・既存のテストに出てくる綴りと、
/// 完全一致が接頭辞より優先される記号（`m`・`min`・`h`・`T`・`G`・`cd`・`Pa`・`mAh` など）を含む。
#[test]
fn spellings_valid_before_keep_their_meaning() {
    const BEFORE: &[(&str, f64, &str)] = &[
        // 基本・組立単位と、完全一致が先に決まる記号
        ("m", 1.0, "m"),
        ("min", 60.0, "s"),
        ("h", 3600.0, "s"),
        ("d", 86_400.0, "s"),
        ("day", 86_400.0, "s"),
        ("yr", 31_557_600.0, "s"),
        ("T", 1.0, "kg/(s^2*A)"),
        ("G", 1e-4, "kg/(s^2*A)"),
        ("cd", 1.0, "cd"),
        ("Pa", 1.0, "kg/(m*s^2)"),
        ("mol", 1.0, "mol"),
        ("mAh", 3.6, "A*s"),
        ("mL", 1e-6, "m^3"),
        ("nT", 1e-9, "kg/(s^2*A)"),
        ("uT", 1e-6, "kg/(s^2*A)"),
        ("kbps", 1e3, "bit/s"),
        ("kg", 1.0, "kg"),
        ("g", 1e-3, "kg"),
        ("g0", 9.806_65, "m/s^2"),
        ("Wh", 3600.0, "kg*m^2/s^2"),
        ("Ah", 3600.0, "A*s"),
        ("AU", 149_597_870_700.0, "m"),
        ("L", 1e-3, "m^3"),
        ("bar", 1e5, "kg/(m*s^2)"),
        // デモの設計データ・文書に出てくる接頭辞付きの綴り
        ("mm", 1e-3, "m"),
        ("cm^2", 1e-4, "m^2"),
        ("km", 1e3, "m"),
        ("mW", 1e-3, "kg*m^2/s^3"),
        ("mN", 1e-3, "kg*m/s^2"),
        ("mN*s", 1e-3, "kg*m/s"),
        ("mN*m", 1e-3, "kg*m^2/s^2"),
        ("mN*m*s", 1e-3, "kg*m^2/s"),
        ("MPa", 1e6, "kg/(m*s^2)"),
        ("GPa", 1e9, "kg/(m*s^2)"),
        ("MHz", 1e6, "1/s"),
        ("kHz", 1e3, "1/s"),
        ("GHz", 1e9, "1/s"),
        ("mHz", 1e-3, "1/s"),
        ("kbit/s", 1e3, "bit/s"),
        ("Mbit/s", 1e6, "bit/s"),
        ("GB", 8e9, "bit"),
        ("MB", 8e6, "bit"),
        ("kB", 8e3, "bit"),
        ("Mbps", 1e6, "bit/s"),
        ("kV", 1e3, "kg*m^2/(s^3*A)"),
        ("mV", 1e-3, "kg*m^2/(s^3*A)"),
        ("uV", 1e-6, "kg*m^2/(s^3*A)"),
        ("mA", 1e-3, "A"),
        ("kA", 1e3, "A"),
        ("mT", 1e-3, "kg/(s^2*A)"),
        ("mPa", 1e-3, "kg/(m*s^2)"),
        ("kPa", 1e3, "kg/(m*s^2)"),
        ("kWh", 3.6e6, "kg*m^2/s^2"),
        ("MWh", 3.6e9, "kg*m^2/s^2"),
        ("mWh", 3.6, "kg*m^2/s^2"),
        ("mg", 1e-6, "kg"),
        ("ug", 1e-9, "kg"),
        ("Gg", 1e6, "kg"),
        ("ms", 1e-3, "s"),
        ("us", 1e-6, "s"),
        ("ns", 1e-9, "s"),
        ("µm", 1e-6, "m"),
        ("um", 1e-6, "m"),
        ("nm", 1e-9, "m"),
        ("pF", 1e-12, "s^4*A^2/(kg*m^2)"),
        ("uF", 1e-6, "s^4*A^2/(kg*m^2)"),
        ("mF", 1e-3, "s^4*A^2/(kg*m^2)"),
        ("nH", 1e-9, "kg*m^2/(s^2*A^2)"),
        ("mH", 1e-3, "kg*m^2/(s^2*A^2)"),
        ("kohm", 1e3, "kg*m^2/(s^3*A^2)"),
        ("Mohm", 1e6, "kg*m^2/(s^3*A^2)"),
        ("mohm", 1e-3, "kg*m^2/(s^3*A^2)"),
        ("mS", 1e-3, "s^3*A^2/(kg*m^2)"),
        ("kGy", 1e3, "m^2/s^2"),
        ("mGy", 1e-3, "m^2/s^2"),
        ("mSv", 1e-3, "m^2/s^2"),
        ("uSv", 1e-6, "m^2/s^2"),
        ("mmol", 1e-3, "mol"),
        ("mrad", 1e-3, "rad"),
        ("urad", 1e-6, "rad"),
        ("µrad", 1e-6, "rad"),
        ("nrad", 1e-9, "rad"),
        ("crad", 1e-2, "rad"),
        ("msr", 1e-3, "rad^2"),
        ("mbar", 1e2, "kg/(m*s^2)"),
        ("kpsi", 6.894_757_293_168e6, "kg/(m*s^2)"),
        ("keV", 1.602_176_634e-16, "kg*m^2/s^2"),
        ("ug0", 9.806_65e-6, "m/s^2"),
        ("mg0", 9.806_65e-3, "m/s^2"),
        ("mdeg", 1.745_329_251_994_33e-5, "rad"),
        ("marcsec", 4.848_136_811_095_36e-9, "rad"),
        ("Myr", 31_557_600.0e6, "s"),
        ("kW", 1e3, "kg*m^2/s^3"),
        ("MW", 1e6, "kg*m^2/s^3"),
        ("nW", 1e-9, "kg*m^2/s^3"),
        ("uW", 1e-6, "kg*m^2/s^3"),
        ("kN", 1e3, "kg*m/s^2"),
        ("uN", 1e-6, "kg*m/s^2"),
        ("kJ", 1e3, "kg*m^2/s^2"),
        ("nA", 1e-9, "A"),
        ("uA", 1e-6, "A"),
        ("pm", 1e-12, "m"),
        ("kN*s", 1e3, "kg*m/s"),
        ("um/s^2", 1e-6, "m/s^2"),
        ("km^3/s^2", 1e9, "m^3/s^2"),
        ("mA*m^2", 1e-3, "A*m^2"),
        ("ohm/m", 1.0, "kg*m/(s^3*A^2)"),
        ("g/m", 1e-3, "kg/m"),
        ("deg/h^(1/2)", 1.745_329_251_994_33e-2 / 60.0, "rad/s^(1/2)"),
        ("ug0/Hz^(1/2)", 9.806_65e-6, "m/s^2*s^(1/2)"),
        ("kHz^(1/2)", 31.622_776_601_683_79, "s^(-1/2)"),
    ];
    for &(spelling, scale, si) in BEFORE {
        let unit = parse_unit(spelling).unwrap_or_else(|e| panic!("{spelling}: {e:?}"));
        let reference = parse_unit(si).unwrap_or_else(|e| panic!("{si}: {e:?}"));
        assert_eq!(unit.dimension(), reference.dimension(), "{spelling} の次元");
        assert!(close(unit.scale(), scale), "{spelling}: {}", unit.scale());
    }
}

/// 足した接頭辞（テラ・ヘクト・フェムト）。
#[test]
fn new_prefixes_parse() {
    const NEW: &[(&str, f64, &str)] = &[
        ("TB", 8e12, "bit"),
        ("Tbit", 1e12, "bit"),
        ("Tbit/s", 1e12, "bit/s"),
        ("Tbps", 1e12, "bit/s"),
        ("THz", 1e12, "1/s"),
        ("hPa", 1e2, "kg/(m*s^2)"),
        ("hL", 0.1, "m^3"),
        ("fT", 1e-15, "kg/(s^2*A)"),
        ("fs", 1e-15, "s"),
        ("fF", 1e-15, "s^4*A^2/(kg*m^2)"),
        ("Tm", 1e12, "m"),
    ];
    for &(spelling, scale, si) in NEW {
        let unit = parse_unit(spelling).unwrap_or_else(|e| panic!("{spelling}: {e:?}"));
        let reference = parse_unit(si).expect(si);
        assert_eq!(unit.dimension(), reference.dimension(), "{spelling}");
        assert!(close(unit.scale(), scale), "{spelling}: {}", unit.scale());
    }
}

/// 完全一致が先に決まる: 足した接頭辞の文字（`T`・`h`・`f`）の単独の記号は意味が変わらない。
#[test]
fn exact_symbols_win_over_new_prefixes() {
    assert!(close(parse_unit("h").unwrap().scale(), 3600.0));
    assert_eq!(
        parse_unit("T").unwrap().dimension(),
        parse_unit("kg/(s^2*A)").unwrap().dimension()
    );
    assert!(close(
        parse_unit("deg/h").unwrap().scale(),
        1.745_329_251_994_33e-2 / 3600.0
    ));
}

/// 接頭辞を付けない単位と、接頭辞どうしの重ね付け・取り違えは読まない。
#[test]
fn prefixes_are_refused_where_they_make_no_sense() {
    for spelling in [
        "kkg", "mmAh", "kmAh", "mmL", "knT", "kuT", "k1", "kone", "m%", "kppm", "kmin", "mmin",
        "kh", "mh", "Th", "fh", "kd", "kday", "krpm", "katm", "karcmin", "Mkbps", "mkg", "ph",
        // ヘクトは Pa・L だけ
        "hm", "hs", "hg", "hW", // 単位の無い接頭辞、接頭辞の重ね付け
        "k", "kk", "Tk", "ft", "mmm",
    ] {
        assert_eq!(
            parse_unit(spelling),
            Err(UnitError::UnknownUnit),
            "{spelling} は拒否する"
        );
    }
    // affine・対数・大きな rad は従来どおり専用のエラー
    assert_eq!(parse_unit("mdegC"), Err(UnitError::AffineInArithmetic));
    assert_eq!(parse_unit("kdB"), Err(UnitError::LogarithmicInArithmetic));
    assert_eq!(parse_unit("Trad"), Err(UnitError::AmbiguousRadian));
}

/// 表の記号はすべて、接頭辞を付けない側か付けられる側のどちらかに分かれる（`NO_PREFIX` の書き間違いを検出）。
#[test]
fn no_prefix_list_names_only_known_symbols() {
    for symbol in NO_PREFIX {
        assert!(NAMED_UNIT_SYMBOLS.contains(symbol), "{symbol} は表に無い");
    }
    for symbol in HECTO_UNITS {
        assert!(NAMED_UNIT_SYMBOLS.contains(symbol), "{symbol} は表に無い");
        assert!(!NO_PREFIX.contains(symbol), "{symbol}");
    }
}

/// 接頭辞の先頭の文字はすべて異なる（最長一致を考えずに済む）。
#[test]
fn prefix_first_letters_are_distinct() {
    for (index, (prefix, _)) in PREFIXES.iter().enumerate() {
        assert!(
            PREFIXES[index + 1..]
                .iter()
                .all(|(other, _)| other.chars().next() != prefix.chars().next()),
            "{prefix}"
        );
    }
}
