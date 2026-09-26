use super::{PromptDefinition, PromptId};

mod current;
mod knowledge_archive;
mod reference_aware_workbench;
mod foundation;

pub fn prompt_definition(id: PromptId) -> &'static PromptDefinition {
    match id {
        PromptId::KnowledgeArchiveProposal => knowledge_archive::definition(),
        PromptId::WorkPreviewReply | PromptId::WorkPreviewPlanner | PromptId::WorkPreviewFinalizer | PromptId::WorkPreviewInvestigation => reference_aware_workbench::prompt_definition(id),
        PromptId::CaptureDistillation
        | PromptId::WorkLogDistillation
        | PromptId::RunReportDistillation
        | PromptId::PublicationIndex
        | PromptId::SpeculativeSearch
        | PromptId::UserGeneration
        | PromptId::TaskJourneyIncrement => foundation::prompt_definition(id),
        _ => current::prompt_definition(id),
    }
}
