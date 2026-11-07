use super::*;
use crate::parse::parse_unit;
use crate::parse::tests::dimension_of;
use crate::typed::{AngularMomentum, Energy, Torque};

#[test]
fn dimension_names_accept_pascal_and_snake_case() {
    assert_eq!(parse_dimension("Torque"), Ok(Torque::DIM));
    assert_eq!(parse_dimension("torque"), Ok(Torque::DIM));
    assert_eq!(parse_dimension("AngularMomentum"), Ok(AngularMomentum::DIM));
    assert_eq!(
        parse_dimension("angular_momentum"),
        Ok(AngularMomentum::DIM)
    );
    assert_eq!(parse_dimension("Power"), parse_dimension("power"));
    assert_eq!(parse_dimension("Dimensionless"), parse_dimension("one"));
    assert_eq!(
        parse_dimension("current"),
        parse_dimension("ElectricCurrent")
    );
    // 設計データで使う綴りは全て引ける。
    for name in [
        "AngularMomentum",
        "Dimensionless",
        "Energy",
        "Force",
        "Mass",
        "Power",
        "Time",
        "Torque",
        "Voltage",
        "current",
        "voltage",
    ] {
        assert!(parse_dimension(name).is_ok(), "{name} must resolve");
    }
}

#[test]
fn runtime_vocabulary_matches_type_level_dimensions() {
    // typed/aliases.rs の `pub type X = ...;` が型レベル次元の一覧。片側だけに足す事故を防ぐ
    // ため、名前の集合が実行時語彙と一致することをソースから照合する。
    let mut typed_count = 0;
    for line in include_str!("../../typed/aliases.rs").lines() {
        let Some(rest) = line.strip_prefix("pub type ") else {
            continue;
        };
        let Some(name) = rest.split(' ').next() else {
            continue;
        };
        if name.starts_with("__") {
            continue;
        }
        assert!(
            parse_dimension(name).is_ok(),
            "typed::{name} has no runtime name"
        );
        typed_count += 1;
    }
    assert_eq!(
        typed_count,
        DIMENSIONS.len(),
        "runtime vocabulary and typed aliases must have the same size"
    );
}

#[test]
fn torque_and_energy_are_one_and_the_same_dimension() {
    // SI では rad が無次元なので、トルクとエネルギーは次元で分けられない。
    // 「弾ける」と誤解しないよう、弾けないことを明示的に固定する。
    assert_eq!(Torque::DIM, Energy::DIM);
    assert_eq!(parse_dimension("Torque"), parse_dimension("Energy"));
    assert_eq!(dimension_of("N*m"), dimension_of("J"));
    // 次元検査は綴りの取り違えを通す。使う側はこれを前提に単位の綴りで守る。
    assert!(dimension_of("J").is_compatible_with(Torque::DIM));
    assert!(dimension_of("N*m").is_compatible_with(Energy::DIM));
}

#[test]
fn torque_and_energy_are_the_only_colliding_dimension_names() {
    // 次元名の語彙で指数ベクトルが一致する組は Energy と Torque の 1 組だけ。新しい次元を
    // 足したときに、気付かないまま既存の次元と同一になるのを防ぐ。
    for (index, (left_name, left)) in DIMENSIONS.iter().enumerate() {
        for (right_name, right) in DIMENSIONS.iter().skip(index + 1) {
            if left != right {
                continue;
            }
            let known = matches!((*left_name, *right_name), ("Energy", "Torque"));
            assert!(
                known,
                "{left_name} and {right_name} share one dimension; \
                 add it to the known pairs here or give it distinct exponents"
            );
        }
    }
}

#[test]
fn documented_unit_examples_parse_to_their_alias_dimension() {
    // typed/aliases.rs の alias doc が ``for example `X`` の形で挙げる綴りを機械的に拾い、
    // (1) 解析でき (2) その alias の次元になる ことを確かめる。
    // `J/(kg*K)`（括弧は文法に無い）のような doc の嘘を再発させないための固定。
    const MARKER: &str = "for example `";
    let mut pending: Option<&str> = None;
    let mut checked = 0;
    for line in include_str!("../../typed/aliases.rs").lines() {
        if let Some(index) = line.find(MARKER) {
            let rest = &line[index + MARKER.len()..];
            pending = Some(rest.split('`').next().expect("closing backtick"));
            continue;
        }
        let Some(rest) = line.strip_prefix("pub type ") else {
            continue;
        };
        let name = rest.split(' ').next().expect("alias name");
        if name.starts_with("__") {
            continue;
        }
        let Some(spelling) = pending.take() else {
            continue;
        };
        let expected = parse_dimension(name)
            .unwrap_or_else(|error| panic!("typed::{name} has no runtime name: {error:?}"));
        let unit = parse_unit(spelling)
            .unwrap_or_else(|error| panic!("typed::{name} doc unit {spelling:?}: {error:?}"));
        assert_eq!(
            unit.dimension(),
            expected,
            "typed::{name} doc unit {spelling:?} has the wrong dimension"
        );
        checked += 1;
    }
    assert!(
        checked >= 25,
        "only {checked} documented unit examples found"
    );
}
