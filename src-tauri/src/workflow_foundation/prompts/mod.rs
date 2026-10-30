use super::{PromptDefinition, PromptId};

mod current;
mod knowledge_archive;
mod knowledge_revision_chat;
mod reference_aware_workbench;
mod foundation;

pub fn prompt_definition(id: PromptId) -> &'static PromptDefinition {
    match id {
        PromptId::KnowledgeRevisionChat => knowledge_revision_chat::definition(),
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

/// Shared by initial generation and every revision, before untrusted source context.
pub(super) const KNOWLEDGE_WRITING: &str = r#"Knowledge documents preserve useful work and learned information for future reuse. Write about the subject itself, not the act of recording it or an evidence audit. Connect supported work background and goals, actions actually performed, outputs and how they were organized, decision points, alternatives and selection criteria, choices and reasons, and reusable instructions or insights. Select the relevant elements and a natural structure for this subject; do not force these into mandatory headings or invent missing details. Synthesize across the full supplied sources instead of copying a report or enumerating provenance states. Exact quotations belong in source bindings; the surrounding prose may explain their supported meaning in new words. For paraphrased claims, set claimBindings.bodyLocator to the exact prose carrying the claim, while retaining the original statement, epistemicState and verbatim sourceRefs. The paraphrase must preserve meaning, attribution, uncertainty and scope; flag unsupported changes as quality findings. Leave bodyLocator empty when statement itself appears in the body.
State user-reported work directly with brief attribution where it matters. Distinguish proposed work, attempted actions and completed actions when the distinction changes what a reader can do. Never invent execution, approval, success, effectiveness or decision rationale. Preserve consequential failures, counterevidence and limitations at the specific step or conclusion they affect, stating the practical consequence or next check. Missing external corroboration alone does not call for a disclaimer. Avoid blanket defensive commentary such as '검증된 흐름은 아닙니다', 'I recorded X' or 'does not mean Y' when it substitutes for the useful substance. These are writing principles, not forbidden-string filters: retain a specific warning when it materially prevents misuse. Internal quality findings and provenance metadata are not a body template.
Assess every supplied imageCatalog entry by its actual documentary value and the user's intent, not filename or a fixed diagram taxonomy. Preserve original visuals whose spatial relationships, layout or visual evidence matter (including diagrams) using ![meaningful description](exact imageRef) beside the relevant explanation. For a screenshot used only to supply text, integrate the readable content into prose instead. Set article.imageDecisions entries {imageRef,disposition:preserve_visual|text_integrated|omit,rationale,bodyLocator}; text_integrated needs the exact integrated prose in bodyLocator. Omit only with a concrete relevance/redundancy reason; never replace a meaningful visual with a text summary alone. Preserve existing visuals and assessments during revision unless the user's edit or supported content assessment calls for a change. Use current imageCatalog refs and existing attachment identities, never paths, uploads or invented URLs. If no assessment is returned the application retains the original visual. Unreadable pixels do not justify invented contents; retain a useful original with a modest description or explain why it cannot be used.
Example source: The user organized draft email replies by customer in drafts/; sending was proposed but no send was performed. Useful prose: '고객별 답장 초안을 drafts/에 모아 후속 검토에 사용할 수 있게 정리했다. 발송 전에는 수신자와 내용을 확인한다.' Do not rewrite this as '답장을 발송했다' or a generic '검증된 흐름은 아닙니다'. Example source: CSV was chosen over a database for a one-time handoff because the recipient uses spreadsheets. Explain that criterion and choice; do not invent a benchmark or generalize CSV as universally faster."#;
