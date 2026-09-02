# Documentation Guide

Use this guide after every completed task to keep repository documentation aligned with the product.

## Documentation structure

| Location | Purpose | Update when |
| --- | --- | --- |
| `README.md`, `README.ko.md` | Project entry points, setup, and links to major capabilities. | Setup, core workflow, or navigation changes. |
| `docs/product-spirit.md`, `docs/product-spirit.ko.md` | Product principles and experience-level commitments. | A change affects product behavior or a stated principle. |
| `docs/features/README.md`, `docs/features/README.ko.md` | Index of user-facing feature guides. | Adding, removing, or renaming a guide. |
| `docs/features/<feature>.md`, `docs/features/<feature>.ko.md` | User-facing behavior, human-control boundaries, and related specs for one feature. | A feature's UI, workflow, state, or user-visible behavior changes. |
| `specs/<id>-<feature>/` | Historical feature specification and implementation artifacts. | The task explicitly maintains that feature's Spec Kit record. |
| `docs/CONTINUATION.md` | Current handoff context and follow-up work. | A task leaves material unfinished work, risks, or operational context. |

## Completion procedure

1. Identify the change's user-facing behavior, workflow, setup, API contract, and verification impact.
2. Review the matching rows above and update each affected document in the same task.
3. Keep English and Korean counterparts aligned when a paired document exists.
4. Update feature-guide links when a guide is added, moved, or renamed.
5. If no document needs an edit, say that the guide was reviewed and no update was needed in the final handoff.

## Writing standard

- Describe current, observable behavior; do not document planned work as implemented.
- Preserve the human-control boundary: AI can propose and organize, while people review and decide.
- Link to the relevant feature spec where it helps readers trace the implementation.
- Use repository-relative links and meaningful image alt text.
