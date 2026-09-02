# Data Model: Refinement Preview Status

No schema migration is required. This feature derives a bounded response from existing workflow items and `ai_runs`, and keeps Preview attempt state in the browser.

## Prior Context

Represents meaningful information already associated with one Problem or Solution.

| Field | Meaning | Validation |
|-------|---------|------------|
| `entity_type` | `problems` or `features` | Other types are unsupported |
| `entity_id` | Current workflow item identity | Must resolve to an existing item |
| `saved_detail` | Problem detail or Solution intended outcome | Blank, boilerplate-only, or title-equivalent text is not context |
| `history` | Same-item chat and prior refinement records | Other entity IDs and unrelated run kinds are excluded |

## Context Summary

| Field | Type | Validation |
|-------|------|------------|
| `has_context` | Boolean | True only when at least one entry remains after normalization |
| `entries` | Ordered list of Summary Entry | Zero to three entries |

### Summary Entry

| Field | Type | Validation |
|-------|------|------------|
| `label` | Short text | Describes current context, recent discussion, or previous Preview |
| `text` | Plain text | Nonblank, normalized whitespace, escaped by the UI before display |

The combined visible length of all entry `text` values is at most 500 characters. Selection priority is current saved detail, then newest relevant history. If content exceeds the remaining budget, the final included entry ends with an ellipsis.

## Refinement Preview Attempt

Ephemeral browser state for one user action.

| Field | Meaning |
|-------|---------|
| `token` | Unique identity distinguishing retries and late responses |
| `entity_type` | Current Problem or Solution type |
| `entity_id` | Current item identity |
| `status` | `generating`, `ready`, `failed`, or `cancelled` |
| `context_summary` | The response retained for loading and completed Preview states |
| `controller` | Cancellation handle for the in-flight generation request |

## State Transitions

```text
idle
  └─ start Problem/Solution attempt ─> generating
       ├─ context exists ─> loading Preview visible
       ├─ context absent ─> existing button progress remains
       ├─ generation succeeds ─> ready (editable Preview; warning clear)
       ├─ generation fails ─> failed (Preview absent; refinement warning visible)
       └─ user closes / changes item / retries ─> cancelled or superseded

failed
  ├─ retry ─> generating (warning cleared)
  ├─ close refinement modal ─> idle (warning cleared)
  └─ change item ─> idle (warning cleared)
```

Only responses whose token, entity type, and entity ID match the current attempt may change UI state.
