//! 出典の台帳 → 表示用の型（ID 順）。

use super::views::file_of;
use crate::dto::SourceView;
use fune_model::design::{Design, EntityKey};
use fune_store::layout::source_dir;

/// 出典の一覧（`DesignView.sources`）。
pub(crate) fn sources(design: &Design) -> Vec<SourceView> {
    design
        .sources
        .values()
        .map(|s| SourceView {
            link: s.web_link().map(str::to_owned),
            file: file_of(design, EntityKey::Source(s.id.to_string())),
            dir: source_dir(design, s.id.as_str()),
            editable: design.shared_catalog_of(s.id.as_str()).is_none(),
            source: s.clone(),
        })
        .collect()
}

/// カタログ品のデータシート（`datasheet` が指す出典）の Web の URL。
pub(crate) fn datasheet_link(design: &Design, datasheet: Option<&str>) -> Option<String> {
    datasheet
        .and_then(|id| design.sources.get(id))
        .and_then(|s| s.web_link())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::design::{Source, SourceKind};

    /// Web の URL の参照先だけをリンクにする（パスや文書番号はリンクにしない）。データシートは出典の ID から引く。
    #[test]
    fn only_web_references_become_links() {
        let mut design = Design::default();
        for (id, reference) in [
            ("ds_web", "https://example.com/rw.pdf"),
            ("ds_path", "docs/datasheets/rw.pdf"),
            ("tr", "TR-EM-01"),
        ] {
            let source = Source {
                reference: Some(reference.into()),
                ..Source::new(id, SourceKind::Datasheet)
            };
            design.sources.insert(id.into(), source);
        }
        let links: Vec<(String, Option<String>)> = sources(&design)
            .into_iter()
            .map(|v| (v.source.id.to_string(), v.link))
            .collect();
        assert_eq!(
            links,
            [
                ("ds_path".to_owned(), None),
                (
                    "ds_web".to_owned(),
                    Some("https://example.com/rw.pdf".to_owned())
                ),
                ("tr".to_owned(), None),
            ]
        );
        let web = datasheet_link(&design, Some("ds_web"));
        assert_eq!(web.as_deref(), Some("https://example.com/rw.pdf"));
        assert_eq!(datasheet_link(&design, Some("ds_path")), None);
        assert_eq!(datasheet_link(&design, Some("missing")), None);
    }
}
