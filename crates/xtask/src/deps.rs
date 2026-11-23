//! 依存の向きの検査（CONTRIBUTING.md の「依存の向き」）。
//!
//! ワークスペース内の crate が、[`ALLOWED`] に挙げた crate にだけ依存しているか（dev-dependencies は見ない）。

use crate::meta::Package;
use std::collections::BTreeMap;

/// crate ごとに、依存してよいワークスペース内の crate。ここに無い crate を足したら検査が失敗する。
const ALLOWED: &[(&str, &[&str])] = &[
    ("sat-units", &[]),
    ("sat-astro", &["sat-units"]),
    ("sat-checkkey", &[]),
    ("sat-analysis", &["sat-units", "sat-astro", "sat-checkkey"]),
    ("fune-model", &["sat-units", "sat-checkkey"]),
    ("fune-store", &["fune-model"]),
    ("fune-ops", &["fune-model", "fune-store"]),
    (
        "fune-projection",
        &[
            "fune-model",
            "sat-analysis",
            "sat-astro",
            "sat-checkkey",
            "sat-units",
        ],
    ),
    ("fune-report", &["sat-units"]),
    (
        "fune-service",
        &[
            "fune-model",
            "fune-store",
            "fune-ops",
            "fune-projection",
            "fune-report",
            "sat-analysis",
            "sat-astro",
            "sat-checkkey",
            "sat-units",
        ],
    ),
    ("fune-server", &["fune-service"]),
    ("fune-cli", &["fune-service", "fune-server"]),
    ("xtask", &[]),
];

/// 違反を `errors` に積む。
pub fn check(packages: &[Package], errors: &mut Vec<String>) {
    errors.extend(violations(packages, ALLOWED));
}

/// 許可表 `allowed` に照らした違反の文。
fn violations(packages: &[Package], allowed: &[(&str, &[&str])]) -> Vec<String> {
    let allowed: BTreeMap<&str, &[&str]> = allowed.iter().copied().collect();
    let members: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let mut out = Vec::new();
    for pkg in packages {
        let Some(ok) = allowed.get(pkg.name.as_str()) else {
            out.push(format!(
                "crate `{}` is not registered in the dependency rules (ALLOWED in crates/xtask/src/deps.rs)",
                pkg.name
            ));
            continue;
        };
        for dep in &pkg.dependencies {
            let internal = members.contains(&dep.name.as_str());
            let is_dev = dep.kind.as_deref() == Some("dev");
            if internal && !is_dev && !ok.contains(&dep.name.as_str()) {
                out.push(format!(
                    "`{}` must not depend on `{}` (dependency direction in CONTRIBUTING.md)",
                    pkg.name, dep.name
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::Dependency;

    fn package(name: &str, deps: &[(&str, Option<&str>)]) -> Package {
        Package {
            name: name.to_string(),
            dependencies: deps
                .iter()
                .map(|(n, k)| Dependency {
                    name: n.to_string(),
                    kind: k.map(str::to_string),
                })
                .collect(),
        }
    }

    const RULES: &[(&str, &[&str])] = &[("low", &[]), ("high", &["low"])];

    #[test]
    fn allowed_and_external_and_dev_dependencies_pass() {
        let packages = [
            package("low", &[("serde", None), ("high", Some("dev"))]),
            package("high", &[("low", None)]),
        ];
        assert!(violations(&packages, RULES).is_empty());
    }

    #[test]
    fn upward_dependency_and_unregistered_crate_are_rejected() {
        let packages = [package("low", &[("high", None)]), package("high", &[])];
        let errors = violations(&packages, RULES);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("`low` must not depend on `high`"));
        let unknown = [package("extra", &[])];
        assert!(violations(&unknown, RULES)[0].contains("not registered"));
    }

    #[test]
    fn the_real_table_covers_every_workspace_layer() {
        let names: Vec<&str> = ALLOWED.iter().map(|(n, _)| *n).collect();
        for n in [
            "sat-units",
            "sat-astro",
            "sat-analysis",
            "fune-service",
            "xtask",
        ] {
            assert!(names.contains(&n), "{n}");
        }
        // lib は app に依存しない（CONTRIBUTING.md の構成のルール 4）
        for (name, deps) in ALLOWED {
            if name.starts_with("sat-") {
                assert!(deps.iter().all(|d| d.starts_with("sat-")), "{name}");
            }
        }
    }
}
