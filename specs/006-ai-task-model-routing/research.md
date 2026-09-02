# Research: Task-Level AI Model Routing

## Decisions

| Decision | Rationale | Alternative rejected |
|---|---|---|
| Store Default and optional Advanced model tiers | Avoids mapping model names to internal workflow stages | Separate Capture, Problem, Solution, and image fields add unnecessary choices |
| Route by stable task identifier | Each operation has an explicit, reviewable preference | Inferring from entity or route misroutes review and image work |
| Fall back to the Default model | Advanced quality remains optional without reducing availability | Failing an enabled task when the second model is blank |
| Use editable opinionated defaults | Prioritizes advanced reasoning for discussion, refinement, drafting, review, and reports | Force every task to the same model tier |

The initial Advanced defaults are Capture, Problem, and Solution discussion/refinement; Problem and Solution drafting; conflict review; image summary; completion review; and completion report. Workbench organization, completed-Solution discussion, and Problem enrichment default to Default.
