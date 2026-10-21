#!/usr/bin/env python3
"""Compare instruction bytes only; does not estimate tokens or model latency."""
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
BASELINE = sys.argv[1] if len(sys.argv) == 2 else None
if not BASELINE:
    raise SystemExit("Usage: python3 scripts/measure_generation_prompts.py <baseline-ref>")


def at_baseline(path):
    return subprocess.check_output(
        ["git", "show", f"{BASELINE}:{path}"], cwd=ROOT, text=True
    )


def instruction(source, prompt_id):
    fragment = source[source.index("PromptId::" + prompt_id):]
    match = re.search(r'r#"(.*?)"#', fragment, re.S)
    if not match:
        raise ValueError(f"Raw prompt not found: {prompt_id}")
    return match.group(1)


rows = []
for module, prompt_id in [("current", "KnowledgeDraft"),
                          ("reference_aware_workbench", "WorkPreviewFinalizer")]:
    path = f"src-tauri/src/workflow_foundation/prompts/{module}.rs"
    before = instruction(at_baseline(path), prompt_id)
    after = instruction((ROOT / path).read_text(), prompt_id)
    raw_before, raw_after = len(before.encode()), len(after.encode())
    # JSON escaping differs from raw text; only the instruction string is compared.
    encoded = lambda value: len(json.dumps(value, ensure_ascii=False).encode())
    rows.append({"prompt": prompt_id, "before_utf8_bytes": raw_before,
                 "after_utf8_bytes": raw_after, "delta_utf8_bytes": raw_after - raw_before,
                 "delta_json_string_bytes": encoded(after) - encoded(before)})
print(json.dumps({"baseline": BASELINE, "measure": "instruction bytes, not tokens or latency",
                  "prompts": rows}, indent=2))
