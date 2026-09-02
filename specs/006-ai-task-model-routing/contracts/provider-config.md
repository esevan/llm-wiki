# Provider Configuration Contract

## Public configuration

The configuration returned to AI Setup includes the endpoint, Default model, Advanced model, task-to-Advanced-tier selections, the legacy report-language value, and whether a secure API key exists. It never includes the API key or legacy stage-specific model preferences. The report-language field remains readable for compatibility, but new managed Knowledge always uses English canonical Markdown.

## Save configuration

AI Setup submits the endpoint, Default model, Advanced model, task selections, and optional new API key. Older clients may continue submitting report language; it is retained but does not control canonical Knowledge generation. Unknown task identifiers are ignored. Missing task selections receive documented initial defaults when configuration is read.

## Resolution guarantee

Every supported AI task uses the selected Advanced model only when it is configured; otherwise it uses the Default model. This applies independently to image summary and conflict review.
