# Data Model

## ReviewRun

Stores input hashes, phase, status, semantic scope/coverage, candidate progress, findings, timings, cancellation, cache provenance, and errors. Running transitions through search, screen, and review; any phase may cancel/fail. Clear is valid only after all retained candidates finish with sufficient evidence.

## ReviewClaim

Stable ID, kind (requirement, scope, constraint, non-goal, validation), and text.

## EvidencePassage

Stable evidence ID, claim ID, Vault path, source hash, 1-based line range, exact text, lexical/semantic scores, and match source. Path/hash/range/text must match the indexed revision.

## CandidateReview and Finding

Candidate review records screen/strong dispositions. A finding references an evidence ID and contains severity, explanation, and required human resolution; citation fields are inherited from evidence rather than accepted as model-authored text.
