# Problem-centered Workbench

**English** | [한국어](conflict-gated-workflow.ko.md)

> **You talk. The work organizes itself. Organize around problems, not tasks.**

![The Workbench shows lightweight Capture, Problem, Solution, and a highlighted In Progress area](images/02-workbench.png)

## Flow

**View review result** reopens the latest completed report instead of creating another AI job.
While a review is active, it opens Queue without duplicating work. **Run new review** lives in
the Solution card's **More actions** menu for changed Solution or Vault evidence; this is an
explicit new request, not automatic approval. If there is no saved report, the result action
points to that menu instead of starting work. Queue also opens reports saved by earlier versions.

1. **Capture** a thought without classifying it.
2. Use AI conversation and Refinement to understand and review a **Problem**.
3. Approve the Problem, explore a **Solution**, and review its intended outcome, boundaries, and validation criteria.
4. Review Knowledge-backed conflict findings. Only a cited `clear` result can start the Solution.
5. Continue inside the Solution Work Log and completion flow.

There is no Task stage. Explore never changes state, AI drafts remain editable, and every transition
requires human action. Soft deletion removes an item from view without deleting private history or
vault files.

Problem approval responds directly from the card: the action shows an in-progress state, refreshes
the Workbench after success, and leaves a visible error message when approval cannot be saved.
The same delegated interaction path opens next-Solution exploration, queues Conflict Review and
Completion Review, and moves Solutions between proposed and in-progress states.

Related Spec Kit: [002 — Conflict-Gated Workflow](../../specs/002-conflict-gated-workflow/spec.md)
