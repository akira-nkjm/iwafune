//! 名前の変わったキー（`fune_store::profile::renamed`）: 古い名前は model の serde の `alias` で新しい名前と同じ値に読み、
//! 読み込みが `toml.renamed_key` の warning を出し、`fune fmt` が新しい名前に書き換える。

use fune_model::Severity;
use fune_store::FileSet;
use fune_store::format::format_text;
use fune_store::layout::analysis_file;
use fune_store::profile::renamed::RENAMED_KEYS;
use fune_store::read::design_of;

const PROJECT: &str = "[project]\nname = \"t\"\nschema = 2\n";

/// 表のパス（任意の名前は `x`）とキー 1 つの本文。
fn text(table: &[&str], key: &str) -> String {
    let header: Vec<&str> = table
        .iter()
        .map(|t| if *t == "*" { "x" } else { t })
        .collect();
    format!(
        "[{}]\n{key} = {{ value = 237.0, unit = \"W/m^2\" }}\n",
        header.join(".")
    )
}

fn files(path: &str, body: &str) -> FileSet {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    set.insert(path, body);
    set
}

#[test]
fn old_names_read_as_new_names_with_a_warning() {
    for rule in RENAMED_KEYS {
        let path = analysis_file(rule.system);
        let old = design_of(&files(&path, &text(rule.table, rule.old)));
        let new = design_of(&files(&path, &text(rule.table, rule.new)));
        assert!(
            new.load_diagnostics.is_empty(),
            "{:?}",
            new.load_diagnostics
        );
        let warned: Vec<_> = old
            .load_diagnostics
            .iter()
            .map(|d| (d.code.as_str(), d.severity, d.line))
            .collect();
        assert_eq!(warned, [("toml.renamed_key", Severity::Warning, Some(2))]);
        assert_eq!(format!("{:?}", old.analysis), format!("{:?}", new.analysis));
        let formatted = format_text(&path, &text(rule.table, rule.old)).unwrap();
        assert_eq!(
            formatted,
            format_text(&path, &text(rule.table, rule.new)).unwrap()
        );
    }
}

/// 古い名前の残るファイルに設定を書くと、先に新しい名前に書き換える（両方の名前にならない）。
#[test]
fn writing_a_setting_renames_old_keys() {
    use fune_model::value::SettingValue;
    use fune_store::Edits;
    use fune_store::write::entities::set_setting;
    for rule in RENAMED_KEYS {
        let path = analysis_file(rule.system);
        let base = files(&path, &text(rule.table, rule.old));
        let mut edits = Edits::new(&base);
        let table: Vec<String> = rule
            .table
            .iter()
            .map(|t| if *t == "*" { "x" } else { t })
            .map(str::to_owned)
            .chain(["label".to_owned()])
            .collect();
        set_setting(
            &mut edits,
            &path,
            &table,
            Some(&SettingValue::Text("t".into())),
        )
        .unwrap();
        let after = edits.files().get(&path).unwrap();
        assert!(
            after.contains(rule.new) && !after.contains(rule.old),
            "{after}"
        );
        assert!(design_of(edits.files()).load_diagnostics.is_empty());
    }
}
