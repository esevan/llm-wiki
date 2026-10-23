# Automatic Capture organization

**English** | [한국어](capture-auto-distillation.ko.md)

Saving a new Capture commits its text and images immediately. LLM Wiki then starts one durable
background organization operation for that exact saved source. Text-only, image-only, and mixed
Captures follow the same path. Existing Captures are left unchanged and are not backfilled.

The Inbox card remains the same Capture in the same position. While work runs, it shows a quiet
processing state. A current grounded result replaces the card's readable title and body with a
short content transition. The transition does not move the card, change focus, reset input, or
represent a workflow change. Reduced-motion preferences suppress the animation.

## Source and generated content

The original text and every submitted image remain separately inspectable in Refinement. Generated
content has its own revision and job provenance; it does not replace the raw source, add a Task or
Problem, increment a user-facing version count, or move the Capture out of Inbox.

Generated title, content, and context allow at most 120, 20,000, and 10,000 Unicode scalar values
respectively, after trimming outer whitespace. Up to eight explicit requests may contain at most
500 characters each. These are maximums, not target lengths. Organization preserves supported
intent without padding, and the full raw source remains available. Every nonempty generated field
must remain grounded in the source. Text references use zero-based Unicode scalar offsets with
an exclusive end; image references use zero-based indices.

Opening Refinement only reads the existing operation. It never starts another initial organization
job. Chat remains usable while organization is queued or running. A chat message can make the
current context newer; one successor attempt then represents the latest context under the same
logical initial operation.

## Currentness, failure, and retry

Generated content applies only when its source and currentness still match. A direct edit always
wins. If an older useful result arrives afterward, it can appear as an earlier proposal for review
without overwriting the current Capture.

Failures remain visible after restart with a safe message. Retry uses the original registered prompt
and exact source revision, and equivalent retry gestures coalesce while an attempt is active.
Stored validation errors identify the field and rule instead of a generic type/value error. The stored
job result includes bounded failure metadata: source character/image counts, output field lengths,
validation field/rule, and provider finish reason when available. Rejected response text is not
saved. Metadata distinguishes a literal empty JSON object response from an absent response candidate;
a failed job's former default `{}` alone cannot establish what the provider returned. Strict provider
schemas describe the same limits without requiring provider-specific length keywords; local
validation enforces the limits. Provider context/output-token limits can still prevent generation.

Related specification: [Automatic Capture Distillation](../../specs/016-capture-auto-distillation/spec.md).
