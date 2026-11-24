//! この文脈で読める形態の解決済み設計と、解決できない参照の対象。

use super::Context;
use crate::configuration_policy::ConfigurationSource;
use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::path::SettingPath;

impl<'a> Context<'a> {
    /// 指定した解析の形態で読む文脈。
    pub(crate) fn for_analysis(&self, id: &str, explicit: Option<&'a str>) -> Self {
        Self { ..*self }.select_configuration(Some(id), explicit)
    }

    /// 別のケースを使う文脈も基準から解決する。実行時に明示した形態と記録先は引き継ぐ。
    pub(crate) fn for_case(&self, id: &str, case: Option<&str>) -> Result<Self, MissingInputs> {
        let explicit = self
            .configuration
            .filter(|(_, source)| *source == ConfigurationSource::Explicit)
            .map(|(id, _)| id);
        Ok(Self::new(self.design, self.base, case)?
            .with_reads(self.reads)
            .select_configuration(Some(id), explicit))
    }

    /// 解析が実際に使う形態と選択元。
    pub(crate) fn configuration(&self) -> Option<(&'a str, ConfigurationSource)> {
        self.configuration
            .filter(|(id, _)| self.design.configurations.contains_key(*id))
    }

    /// 無効な選択を、不足入力として選択元の欄に結び付ける。
    pub(crate) fn require_configuration(&self) -> Result<(), MissingInputs> {
        let Some((id, source)) = self.configuration else {
            return Ok(());
        };
        if self.configuration().is_some() {
            return Ok(());
        }
        let target = match source {
            ConfigurationSource::Explicit => Some(Target::configuration(id, None)),
            ConfigurationSource::Launch => {
                Some(SettingPath::conditions(&["launch", "configuration"]).into())
            }
            ConfigurationSource::Phase => self
                .phase()
                .map(|p| Target::phase(p.id.as_str(), Some("configuration"))),
        };
        let mut missing = MissingInputs::new();
        if let Some(target) = target {
            missing.push(target, format!("Configuration `{id}` is not defined"));
        }
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_store::{FileSet, read::design_of};

    /// 入れ子のケースが形態を切り替えるとき、基準の解決結果から引き直す。
    #[test]
    fn nested_cases_select_from_base_and_keep_explicit_selection() {
        let mut files = FileSet::new();
        files.insert("fune.toml", "[project]\nname = \"Test\"\n");
        for id in ["STOWED", "DEPLOYED"] {
            files.insert(
                format!("configurations/{id}.toml"),
                "label = \"Configuration\"\n",
            );
            files.insert(
                format!("phases/{id}.toml"),
                format!("configuration = \"{id}\"\n"),
            );
            files.insert(format!("cases/{id}.toml"), format!("phase = \"{id}\"\n"));
        }
        let design = design_of(&files);
        let base = fune_model::resolve(&design);
        let id = "power.energy_balance";
        let stowed = Context::new(&design, &base, Some("STOWED")).unwrap();
        assert!(std::ptr::eq(
            stowed.resolved,
            &base.configurations["STOWED"]
        ));
        let nested = stowed.for_case(id, Some("DEPLOYED")).unwrap();
        assert!(std::ptr::eq(
            nested.resolved,
            &base.configurations["DEPLOYED"]
        ));
        assert!(std::ptr::eq(
            nested.for_case(id, None).unwrap().resolved,
            &base
        ));
        let explicit = stowed.for_analysis(id, Some("STOWED"));
        assert!(std::ptr::eq(
            explicit.for_case(id, Some("DEPLOYED")).unwrap().resolved,
            stowed.resolved
        ));
    }
}
