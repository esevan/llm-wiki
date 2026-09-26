# Quickstart: Validate Multi-aspect Retrieval

## Prerequisites

- Work from the dedicated `018-multiaspect-knowledge-retrieval` worktree.
- Use migration 19 after the integrated A/B/C migration chain through version 18.
- Use the verified bundled embedding assets only when semantic fixture checks require them.

## Focused validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml retrieval
cargo test --manifest-path src-tauri/Cargo.toml --lib v19_
cargo test --manifest-path src-tauri/Cargo.toml --test work_tracking_search
git diff --check
```

Fixtures must demonstrate:

1. Standard YAML front matter and recognized English/Korean headings create bounded explicit units;
   absent/malformed metadata leaves body search intact and never creates a final decision.
2. A semantic-only applicability candidate can enter ranking without any lexical match.
3. Applicability outranks equally relevant body text; explicit unverified ideas remain discoverable
   but rank below applicable confirmed decisions.
4. A superseded decision redirects to a condition-compatible confirmed-final successor while the
   original remains labelled historical; missing targets and cycles do not redirect.
5. Stale revisions, model/version mismatch, dimension mismatch, and malformed vector bytes are
   rejected while lexical fallback remains available.
6. Results remain stable by identity under duplicate titles; path moves preserve identity only with
   explicit `document_id`; no more than eight passages / 6,000 estimated tokens are returned.

## Broader integration checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Record the warm lexical and structural-index timing fixtures separately from embedding inference.
The final local debug-profile fixture indexed 1,000 notes / about 10 MB in 1.939 seconds
(1.093 seconds scan/hash/parse and 0.845 seconds SQLite persistence); warm lexical search was
11.906 milliseconds. Embedding inference was excluded and remains model- and hardware-dependent.
Packaged desktop E2E is unnecessary unless later API/UI integration exposes a concrete native-boundary
risk that focused tests cannot cover.
