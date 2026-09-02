# Data Model: Task-Level AI Model Routing

## Model Tier Configuration

| Field | Meaning | Validation |
|---|---|---|
| Endpoint | OpenAI-compatible provider location | Required configuration value |
| Default model | Model used by ordinary tasks and every fallback | Required before an AI task can run |
| Advanced model | Optional model used by selected quality-sensitive tasks | May be blank |
| API-key reference | Secure credential location | Never returned in public configuration |
| Report language | Existing report output preference | Korean or English |

## Task-Tier Preference

| Field | Meaning | Validation |
|---|---|---|
| Task identifier | Stable named AI operation | Must be one of the supported operations |
| Use Advanced | Whether the operation prefers the Advanced model | Boolean |

## Model Resolution

1. Read the task preference.
2. If the task is selected and an Advanced model exists, use the Advanced model.
3. Otherwise use the Default model.
4. If no Default model exists, ask the person to configure AI Setup before making a provider request.

## Supported Tasks

Capture discussion/refinement; Capture-to-Problem drafting; Problem discussion/refinement; workbench organization; Problem-to-Solution drafting; Solution discussion/refinement; completed-Solution discussion; conflict review; image summary; completion review; completion report; and Problem enrichment.
