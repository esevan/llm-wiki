use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Organization {
    pub(crate) outcome: String,
    pub(crate) path: String,
    pub(crate) rationale: String,
    #[serde(default)]
    pub(crate) target_path: Option<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(default)]
    pub(crate) moc_paths: Vec<String>,
    #[serde(default)]
    pub(crate) new_category_rationale: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Artifact {
    pub(crate) document_id: String,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) bytes: Option<String>,
    pub(crate) sha256: Option<String>,
    pub(crate) expected_hash: Option<String>,
    #[serde(default)]
    pub(crate) idea_id: Option<String>,
    #[serde(default)]
    pub(crate) idea_revision: Option<i64>,
}
impl Organization {
    pub(crate) fn validate(&self) -> bool {
        matches!(
            self.outcome.as_str(),
            "new" | "update" | "merge" | "conflict" | "supersede"
        ) && !self.path.trim().is_empty()
            && !self.rationale.trim().is_empty()
            && self.tags.len() <= 24
            && self.aliases.len() <= 24
            && self.moc_paths.len() <= 8
    }
}
