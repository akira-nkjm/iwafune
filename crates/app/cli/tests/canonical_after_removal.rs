//! 「Op が書いたファイルは、いつも正規形」を、キー・表・行を消す編集も含めて確かめる（#382）。
//! デモの一時コピーに `fune` のコマンドを順に当て、コマンドごとに `fune fmt --check` が通ることを見る。
//! 各場面は「消す前に、消す行が表でいちばん長いキーになるようにする」準備と、消す編集からなる。

mod common;

use common::{demo_copy, fune, run_cli};

/// 場面 1 つ（名前、コマンドの列）。
type Scenario = (&'static str, &'static [&'static [&'static str]]);

const MANEUVER_ADD: &[&str] = &[
    "maneuver",
    "add",
    "T1",
    "--category",
    "insertion",
    "--method",
    "impulsive",
    "--thruster",
    "EPT",
    "--after",
    "DEORBIT",
    "--perigee-altitude",
    "300 km",
    "--apogee-altitude",
    "33000 km",
    "--inclination",
    "30 deg",
];

const SCENARIOS: &[Scenario] = &[
    // マヌーバ: [target] のいちばん長いキーを消す（#382 の再現）／トップのキーを消す／丸ごと消す
    (
        "maneuver target key",
        &[
            MANEUVER_ADD,
            &[
                "maneuver",
                "set",
                "T1",
                "--category",
                "insertion",
                "--method",
                "impulsive",
                "--thruster",
                "EPT",
                "--after",
                "DEORBIT",
                "--apogee-altitude",
                "33000 km",
                "--inclination",
                "30 deg",
            ],
        ],
    ),
    // マヌーバ: 既にあるファイルに、新しい表（[transfer]）を足す
    (
        "maneuver gains a table",
        &[
            MANEUVER_ADD,
            &[
                "maneuver",
                "set",
                "T1",
                "--category",
                "insertion",
                "--method",
                "impulsive",
                "--thruster",
                "EPT",
                "--after",
                "DEORBIT",
                "--body",
                "mars",
                "--perigee-altitude",
                "300 km",
                "--apogee-altitude",
                "33000 km",
                "--depart",
                "2028-12-10T00:00:00Z",
                "--arrive",
                "2029-09-25T00:00:00Z",
            ],
        ],
    ),
    (
        "maneuver top-level key",
        &[
            MANEUVER_ADD,
            &[
                "maneuver",
                "set",
                "T1",
                "--category",
                "insertion",
                "--method",
                "impulsive",
                "--perigee-altitude",
                "300 km",
                "--apogee-altitude",
                "33000 km",
                "--inclination",
                "30 deg",
            ],
        ],
    ),
    ("maneuver rm", &[MANEUVER_ADD, &["maneuver", "rm", "T1"]]),
    // 部品の属性の上書き（[attr]）と状態ごとの値
    (
        "unset part attribute",
        &[
            &["set", "RW_X.mass", "620 g"],
            &["set", "RW_X.size", "50 50 30 mm"],
            &["unset", "RW_X.size"],
            &["unset", "RW_X.mass"],
        ],
    ),
    (
        "unset part attribute per state",
        &[
            &["set", "RW_X.power.on", "3 W"],
            &["set", "RW_X.power.idle", "1 W"],
            &["unset", "RW_X.power.on"],
        ],
    ),
    (
        "unset part label",
        &[&["set", "RW_X.label", "x"], &["unset", "RW_X.label"]],
    ),
    // 設定
    (
        "unset setting (analysis)",
        &[&["setting", "unset", "analysis.power.conversion_efficiency"]],
    ),
    (
        "unset setting (orbit)",
        &[&["setting", "unset", "conditions.orbit.sun_synchronous"]],
    ),
    (
        "unset setting (ground link)",
        &[&[
            "setting",
            "unset",
            "conditions.ground.TOKYO_GS.link.uhf.noise_temperature",
        ]],
    ),
    (
        "unset setting (ground)",
        &[&[
            "setting",
            "unset",
            "conditions.ground.TOKYO_GS.min_elevation",
        ]],
    ),
    (
        "unset setting (budget)",
        &[&["setting", "unset", "budget.policy.harness.allowance"]],
    ),
    // フェーズ: 軌道の上書きのキー・軌道ごと・姿勢則
    (
        "phase unset orbit key",
        &[
            &["phase", "unset-field", "DISPOSAL", "orbit.eccentricity"],
            &["phase", "unset-field", "DISPOSAL", "orbit.mean_anomaly"],
        ],
    ),
    (
        "phase unset orbit",
        &[&["phase", "unset-orbit", "DISPOSAL"]],
    ),
    (
        "phase unset law",
        &[
            &[
                "phase",
                "set-field",
                "LEOP",
                "law.NOMINAL.law",
                "nadir_pointing",
            ],
            &["phase", "unset-law", "LEOP", "NOMINAL"],
        ],
    ),
    (
        "phase set drops keys",
        &[&[
            "phase", "set", "DISPOSAL", "--order", "3", "--modes", "SAFE",
        ]],
    ),
    ("phase rm", &[&["phase", "rm", "DISPOSAL"]]),
    // モード: 部品の行・時間・タイミング
    (
        "mode unset row",
        &[
            &["mode", "unset", "NOMINAL", "HTR_BAT"],
            &["mode", "unset", "NOMINAL", "HTR_CAM"],
        ],
    ),
    (
        "mode duration and timing",
        &[
            &["mode", "timing", "NOMINAL", "--when", "sunlit"],
            &["mode", "duration", "NOMINAL"],
            &["mode", "timing", "NOMINAL"],
        ],
    ),
    (
        "mode rm",
        &[
            &["mode", "add", "TMP", "--from", "NOMINAL"],
            &["mode", "rm", "TMP"],
        ],
    ),
    // カタログ
    (
        "catalog unset field",
        &[&["catalog", "unset", "rw_10mNms", "manufacturer"]],
    ),
    (
        "catalog unset value",
        &[&["catalog", "unset", "rw_10mNms", "size"]],
    ),
    (
        "catalog unset value per state",
        &[&["catalog", "unset", "rw_10mNms", "power.on"]],
    ),
    // 要求
    (
        "requirement set drops fields",
        &[&[
            "requirement",
            "set",
            "R_LINK_MARGIN",
            "--statement",
            "Margin.",
            "--quantity",
            "Dimensionless",
            "--kind",
            "at_least",
            "--value",
            "3 dB",
            "--source",
            "sys_judgement",
            "--analysis",
            "comm.link_budget",
            "--check",
            "margin",
        ]],
    ),
    ("requirement rm", &[&["requirement", "rm", "R_MASS"]]),
    // ケース
    (
        "case set drops keys",
        &[&[
            "case",
            "set",
            "hot_imaging",
            "--mode",
            "MISSION",
            "--side",
            "max",
        ]],
    ),
    (
        "case unset phase",
        &[&["case", "unset-phase", "hot_imaging"]],
    ),
    // ネット・ハーネス
    ("net unset field", &[&["net", "unset", "P5V", "source"]]),
    ("net unset rail", &[&["net", "unset", "P5V", "rail"]]),
    ("net disconnect", &[&["net", "disconnect", "CAM.PWR"]]),
    (
        "harness set drops keys",
        &[&[
            "harness",
            "set",
            "H_OBC_CAM",
            "--from",
            "OBC.SPW1",
            "--to",
            "CAM.DATA",
        ]],
    ),
    // 部品の取付
    ("part place root", &[&["part", "place", "CAM", "--root"]]),
    // プロジェクト
    ("project unset label", &[&["project", "unset", "label"]]),
    // 通信の回線・地上局・シーケンス
    ("link rm", &[&["link", "rm", "uhf_up"]]),
];

#[test]
fn files_written_by_removing_edits_are_canonical() {
    let mut failures = Vec::new();
    for (name, steps) in SCENARIOS {
        let (_tmp, root) = demo_copy();
        for (i, step) in steps.iter().enumerate() {
            run_cli(&root, step);
            let out = fune()
                .arg("-C")
                .arg(&root)
                .args(["fmt", "--check"])
                .output()
                .unwrap();
            if !out.status.success() {
                failures.push(format!(
                    "{name} (step {}: fune {}): {}",
                    i + 1,
                    step.join(" "),
                    String::from_utf8_lossy(&out.stdout).trim()
                ));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "not canonical after an Op:\n{}",
        failures.join("\n")
    );
}
