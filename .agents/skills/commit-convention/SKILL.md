---
name: commit-convention
description: >-
  Create and validate repository Git commits and prepare branches for
  push using the required English gitmoji message format and
  single-commit PR convention. Use whenever composing, amending,
  squashing, or pushing commits in this repository.
---

# Commit Convention

Apply these rules to every Git commit:

- Write the complete commit message in English.
- Keep the subject at 50 characters or fewer, including the gitmoji.
- Start the subject with an appropriate gitmoji.
- Phrase the subject as an imperative and emphasize what changed.
- Use the body to explain why the change was needed, how it works, and what
  verification was performed. Report checks actually run and any material
  verification limitations; do not invent successful checks.
- Wrap body lines at 72 characters. Do not wrap URLs or indivisible
  tokens.
- Omit the body only for a trivial typo or wording correction whose reason and
  effect are fully clear from the subject. Bug fixes, behavior changes, build or
  configuration changes, and refactors always require a body.
- A concise subject and the single-commit PR rule do not justify omitting the
  body. The latter limits commit count, not message length.

When the commit introduces a breaking change, finish the body with a
`BREAKING CHANGE:` section. Explain the incompatibility and its impact.
If users must take action, add a `Required Actions:` subsection with
concrete migration steps.

Treat every remote push as preparation for a pull request. Before
pushing, ensure the branch contains exactly one commit for its complete
change relative to the intended target branch. Squash locally when
necessary and make the resulting commit comply with this convention. Do
not push unless the current task authorizes it. If making the remote
branch single-commit requires a history-rewriting push, verify the exact
branch and obtain any authorization required for that destructive
operation.

Before committing or pushing, inspect the final subject, required body content, body wrapping,
breaking-change footer, and commit count. Preserve repository-specific
Git and worktree instructions that impose additional constraints.
