use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PublicationState {
    NotRequested,
    Offered,
    Deferred,
    Draft,
    Publishing,
    Published,
    Conflicted,
    Failed,
}

impl PublicationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::Offered => "offered",
            Self::Deferred => "deferred",
            Self::Draft => "draft",
            Self::Publishing => "publishing",
            Self::Published => "published",
            Self::Conflicted => "conflicted",
            Self::Failed => "failed",
        }
    }
}
