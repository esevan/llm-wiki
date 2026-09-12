/**
 * Evidence-based legacy classification for the Task artifact.  These are not
 * coverage exemptions: `frontend/index.html` still loads the runtime files,
 * but Task Workbench omits #board and therefore has no live entrypoint for
 * board-only actions. Any future entrypoint must move that family to reachable
 * and add individual manifest/scenario records.
 */
export const legacyReachability = {
  F48: { reachable: false, evidence: 'feature/manual/draft dialogs are opened only by legacy board next-chat actions; the React migrated Problem card exposes only mode=refine and no next-draft entrypoint' },
  F49: { reachable: true, evidence: 'the React migrated Problem Refine action opens the shipping legacy chat dialog and preview controls' },
  F50: { reachable: true, evidence: 'notice confirmation is used by reachable shared runtime operations and ships in OverlayLayer' },
  F51: { reachable: false, evidence: 'transition modal ships, but every openTransition caller is rendered only by legacy board/detail code and Task Workbench exposes none' },
  F60: { reachable: false, evidence: 'board-only workbench actions require #board; Task Workbench intentionally omits it' },
  F61: { reachable: false, evidence: 'legacy Problem cards require #board; Task Workbench has no Problem-card entrypoint' },
  F62: { reachable: false, evidence: 'legacy Solution actions require rendered legacy board cards' },
  F63: { reachable: false, evidence: 'Solution Work composer requires legacy feature detail opened from board' },
  F64: { reachable: false, evidence: 'legacy capture/organize/flow handlers require old board shell' },
  F65: { reachable: false, evidence: 'archive actions are rendered from legacy workbench context absent in Task Workbench' },
  F66: { reachable: false, evidence: 'completed workspace opens from legacy Solution detail absent in Task entrypoint' },
  F67: { reachable: true, evidence: 'completed Conflict Review jobs and notifications open the shared decision surface from the reachable queue' },
  F68: { reachable: true, evidence: 'cross-dialog close/outside/focus handlers ship in OverlayLayer/foundation runtime' },
} as const;

/** Exact source signatures reviewed as old-board-only. Broad wildcard exemptions are forbidden. */
export const legacyReviewedUnreachableSourceEvidence: Readonly<Record<string, readonly string[]>> = {
  'frontend/src/features/overlays/OverlayLayer.tsx': [
    'id="feature-title"', 'id="feature-outcome"', 'id="feature-nongoals"', 'data-control="feature-save"',
    'id="feature-cancel"',
    'id="draft-main"', 'id="draft-detail"', 'id="draft-extra"', 'id="draft-submit"',
    'id="draft-cancel"',
    'id="manual-title"', 'id="manual-detail"', 'id="manual-localized-only"', 'data-control="manual-save"',
    'id="manual-cancel"', 'id="transition-submit"', 'id="transition-cancel"',
    'id="notice-cancel"',
  ],
  'frontend/public/runtime/archive.js': [
    'data-archive-path', 'data-completed-id', 'archive-file-icon', 'archive-delete-icon', 'archive-regenerate-icon',
  ],
  'frontend/public/runtime/completed-workspace.js': [
    'id="preview-work-tab"', 'id="preview-archive-tab"', 'data-chat-prompt', 'id="create-follow-up-problem"',
    'data-completed-action', 'data-lineage-evidence', 'class="lineage-details"', 'class="lineage-correction"',
    'aria-label="Correct AI interpretation"', 'class="tiny hot" type="submit"', 'class="lineage-stage"',
    'class="lineage-reference-chip"', 'class="lineage-reference-close"', 'id="completion-reason"',
    'data-tooltip="Complete Problem"',
  ],
  'frontend/public/runtime/conflicts.js': [
    '<details', 'name="conflict-resolution-', 'aria-label="\'+esc(conflictCopy', 'onclick="cancelConflictReview()',
    'id="conflict-review-continue"',
  ],
  'frontend/public/runtime/solution-work.js': [
    'data-summarize-entry', 'data-comment-entry', 'placeholder="Add a comment"', 'data-check-id', 'data-check-text',
    '<button class="tiny">',
    'id="progress-body"', 'data-tooltip="Save progress"', 'placeholder="Add a checklist item"',
    '<details open', 'id="feature-validation"', 'id="draft-validation"', 'data-archive-path', 'data-completed-id',
  ],
  'frontend/public/runtime/transitions.js': [
    'name="\'+esc(field.name)',
  ],
  'frontend/public/runtime/workbench.js': [
    'data-tooltip="\'+esc(label)', 'class="\'+kind', 'class="card-menu"', 'data-solution-action', 'data-archive-path',
    'data-completed-id', 'data-retry-knowledge', 'archive-regenerate-icon', 'data-tooltip="Regenerate Lineage and completed-work document"',
    'role="button" aria-label="Open details"',
  ],
};
