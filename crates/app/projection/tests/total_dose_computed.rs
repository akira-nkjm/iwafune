//! `orbit.total_dose` の環境の表: 利用者の表が無い区間は、放射線帯と SHIELDOSE-2 のデータがそろっていれば捕捉粒子の計算（`orbit.trapped_dose`）の表、
//! 利用者の表があれば**必ずそちら**。データはキャッシュ（`IWAFUNE_DATA_DIR`）から読んで sat-astro に登録する（無ければ読み飛ばす）。
//! 登録はプロセスで一度きりなので、データを読む試験はこのファイルだけに置く。

use fune_model::Design;
use fune_model::value::{Curve, Scalar, Value};
use fune_projection::{Context, orbit};
use sat_analysis::orbit::EnvironmentSource;
use sat_astro::geomag::GaussModel;
use sat_astro::radiation::registry::{install_field, install_map, install_shielding};
use sat_astro::radiation::shielding::ShieldingData;
use sat_astro::radiation::{BeltField, TrappedMap, TrappedModel};
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn read(name: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("IWAFUNE_DATA_DIR").filter(|d| !d.is_empty())?;
    std::fs::read(Path::new(&dir).join(name)).ok()
}

/// 配布データを読んで登録する。そろわなければ `false`。
fn install() -> bool {
    let files = [
        "ap8min.asc",
        "ap8max.asc",
        "ae8min.asc",
        "ae8max.asc",
        "jensen_cain_62.dat",
        "gsfc66.dat",
        "shieldose2.dat",
    ];
    let Some(bytes) = files.iter().map(|f| read(f)).collect::<Option<Vec<_>>>() else {
        eprintln!("SKIPPED: the trapped-radiation data is not in IWAFUNE_DATA_DIR");
        return false;
    };
    for (model, b) in TrappedModel::ALL.iter().zip(&bytes) {
        install_map(*model, TrappedMap::from_ascii_bytes(b).expect("map")).expect("install map");
    }
    let jc = GaussModel::from_jensen_cain_bytes(&bytes[4]).expect("jensen cain");
    install_field(BeltField::JensenCain1960, jc).expect("field");
    let gsfc = GaussModel::from_gsfc1266_bytes(&bytes[5], 1970.0).expect("gsfc");
    install_field(BeltField::Gsfc1266To1970, gsfc).expect("field");
    install_shielding(ShieldingData::from_bytes(&bytes[6]).expect("shieldose2")).expect("install");
    true
}

fn set(d: &mut Design, part: &str, attr: &str, value: f64, unit: &str) {
    let p = d.parts.get_mut(part).expect("part");
    p.attr.insert(
        fune_model::Id::new(attr),
        fune_model::value::Authored::bare(Value::Scalar(Scalar::new(value, unit))),
    );
}

fn input(d: &Design) -> sat_analysis::orbit::TotalDoseInput {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, None).expect("context");
    orbit::total_dose(&ctx).expect("input")
}

#[test]
fn computed_environment_fills_legs_without_a_table_and_a_user_table_wins() {
    if !install() {
        return;
    }
    let mut d = demo();
    set(&mut d, "OBC", "tid_tolerance", 0.1, "kGy");
    set(&mut d, "OBC", "shielding", 2.0, "mm");
    // 表を書かない設計: 期間のある区間はすべて計算した表（捕捉粒子の悪い方）
    let started = std::time::Instant::now();
    let i = input(&d);
    eprintln!(
        "projection with the computed environment took {:?}",
        started.elapsed()
    );
    let with_duration: Vec<_> = i.legs.iter().filter(|l| l.duration.is_some()).collect();
    assert!(!with_duration.is_empty());
    for l in &with_duration {
        assert_eq!(
            l.environment_source,
            Some(EnvironmentSource::ComputedTrapped)
        );
        let table = l.environment.as_ref().expect("table");
        assert!(table.len() >= 3);
        assert!(
            table
                .windows(2)
                .all(|w| w[1].shielding.value() > w[0].shielding.value())
        );
        assert!(table.iter().all(|p| p.rate.value() > 0.0));
    }
    // 期間の無い区間は計算しない
    assert!(
        i.legs
            .iter()
            .filter(|l| l.duration.is_none())
            .all(|l| l.environment.is_none())
    );
    // 解析も走り、出どころが出力に出る
    let out =
        <sat_analysis::orbit::TotalDose as sat_analysis::spec::Analysis>::run(&i).expect("run");
    assert!(
        out.legs
            .iter()
            .any(|l| l.environment_source == Some(EnvironmentSource::ComputedTrapped))
    );
    assert!(out.checks.iter().any(|c| c.key == "environment_source"));
    // 利用者の表を書けば、それが必ず使われる
    d.conditions.orbit.as_mut().unwrap().dose_rate_by_shielding = Some(Curve {
        at: vec![1.0, 10.0],
        at_unit: "mm".into(),
        values: vec![100.0, 10.0],
        unit: "Gy/yr".into(),
    });
    let i = input(&d);
    let base = i
        .legs
        .iter()
        .find(|l| l.phase.as_deref() == Some("NOMINAL_OPS"))
        .expect("leg");
    assert_eq!(base.environment_source, Some(EnvironmentSource::UserTable));
    let rate = base.environment.as_ref().unwrap()[0].rate.value();
    assert!((rate - 100.0 / (365.25 * 86_400.0)).abs() < 1e-12, "{rate}");
}
