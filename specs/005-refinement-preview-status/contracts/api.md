# API Contract: Refinement Context

## Read a Problem or Solution context summary

`GET /api/{entity_type}/{entity_id}/refinement-context`

### Path parameters

| Name | Allowed values |
|------|----------------|
| `entity_type` | `problems`, `features` |
| `entity_id` | Existing item ID of the selected type |

### Success response — `200 OK`

```json
{
  "has_context": true,
  "entries": [
    {
      "label": "Current context",
      "text": "People lose time locating the approved decision."
    },
    {
      "label": "Recent discussion",
      "text": "The most important boundary is avoiding a migration project."
    }
  ]
}
```

Contract rules:

- `entries` contains zero to three items.
- The combined visible length of all `text` values does not exceed 500 characters.
- `has_context` is true if and only if `entries` is non-empty.
- Entries contain only context belonging to the requested item.
- No model/provider or vault operation is performed by this endpoint.

### No-context response — `200 OK`

```json
{
  "has_context": false,
  "entries": []
}
```

### Errors

- `400 Bad Request`: unsupported entity type, including `captures`.
- `404 Not Found`: requested Problem or Solution does not exist.

The browser treats failure of this read as unavailable loading context, not as authorization to apply or invent content. Failure of the separate refinement-generation request still follows the Preview failure UI contract.

## Existing refinement proposal

`POST /api/{entity_type}/{entity_id}/refine` remains the proposal-generation contract. Its success and error shapes remain compatible. The browser retains the context-read response for both the loading and completed Preview rather than requiring a second context read.
