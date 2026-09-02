from __future__ import annotations

import hashlib
import json
import re
from typing import Any, Iterable

LINEAGE_SCHEMA_VERSION = 1
ABSENT_REASON = "Not explicitly recorded"


def _row(row: Any) -> dict[str, Any]:
    return dict(row) if row is not None else {}


def _rows(rows: Iterable[Any]) -> list[dict[str, Any]]:
    return [dict(row) for row in rows]


def _digest(value: object) -> str:
    payload = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def _concise(value: object, limit: int = 420) -> str:
    text = re.sub(r"\s+", " ", str(value or "")).strip()
    if not text:
        return ABSENT_REASON
    return text if len(text) <= limit else text[: limit - 1].rstrip() + "…"


def _section(value: str, *names: str) -> str:
    current = ""
    found: list[str] = []
    wanted = {name.casefold() for name in names}
    for raw in str(value or "").splitlines():
        line = raw.strip()
        heading = re.match(r"^(?:#{1,6}\s*)?(?:\*\*)?([^:#*]{2,48})(?:\*\*)?\s*:?\s*$", line)
        if heading and (line.startswith("#") or line.endswith(":") or line.startswith("**")):
            current = re.sub(r"\s+", " ", heading.group(1)).strip().casefold()
            continue
        if current in wanted and line:
            found.append(line)
    return " ".join(found).strip()


def validate_inference_payload(value: object, evidence_ids: set[str]) -> list[dict[str, object]]:
    if not isinstance(value, dict) or not isinstance(value.get("claims"), list):
        raise ValueError("Lineage inference must contain a claims list")
    validated: list[dict[str, object]] = []
    for index, raw in enumerate(value["claims"]):
        if not isinstance(raw, dict):
            raise ValueError(f"Lineage inference claim {index + 1} is invalid")
        text = str(raw.get("text", "")).strip()
        confidence = str(raw.get("confidence", "")).strip().lower()
        cited = [str(item) for item in raw.get("evidence_ids", []) if str(item)]
        if not text or confidence not in {"high", "medium", "low"} or not cited:
            raise ValueError("Every inferred claim needs text, confidence, and evidence IDs")
        if any(item not in evidence_ids for item in cited):
            raise ValueError("Lineage inference cited evidence outside the supplied bundle")
        if re.search(r"\b(?:addressed|resolved)\b", text, re.I):
            raise ValueError("AI inference cannot establish an Addressed or Resolved conflict")
        validated.append(
            {
                "claim_key": str(raw.get("claim_key") or f"inferred:{index + 1}"),
                "text": text,
                "confidence": confidence,
                "evidence_ids": cited,
            }
        )
    return validated[:12]


def build_lineage_document(db: Any, feature_id: str) -> tuple[dict[str, object], str]:
    feature = _row(db.execute("SELECT * FROM features WHERE id=?", (feature_id,)).fetchone())
    if not feature:
        raise ValueError("Solution not found")
    problem = _row(db.execute("SELECT * FROM problems WHERE id=?", (feature["problem_id"],)).fetchone())
    capture = _row(db.execute("SELECT * FROM captures WHERE id=?", (problem.get("capture_id", ""),)).fetchone())
    completion = _row(db.execute("SELECT * FROM completions WHERE feature_id=?", (feature_id,)).fetchone())
    decisions = _rows(
        db.execute(
            "SELECT * FROM solution_decision_events WHERE feature_id=? ORDER BY created_at,rowid", (feature_id,)
        ).fetchall()
    )
    conflict_reports = _rows(
        db.execute(
            "SELECT * FROM conflict_reports WHERE feature_id=? ORDER BY created_at,rowid", (feature_id,)
        ).fetchall()
    )
    conflict_addresses = _rows(
        db.execute(
            "SELECT * FROM conflict_addresses WHERE feature_id=? ORDER BY created_at,rowid", (feature_id,)
        ).fetchall()
    )
    completion_decisions = _rows(
        db.execute(
            "SELECT * FROM problem_completion_decisions WHERE problem_id=? ORDER BY created_at,rowid",
            (problem.get("id", ""),),
        ).fetchall()
    )
    reviews = _rows(
        db.execute(
            "SELECT * FROM completion_reviews WHERE feature_id=? ORDER BY created_at,rowid", (feature_id,)
        ).fetchall()
    )
    progress = _rows(
        db.execute(
            "SELECT id,feature_id,body,image_summary,created_at FROM solution_progress_entries WHERE feature_id=? ORDER BY created_at,rowid",
            (feature_id,),
        ).fetchall()
    )
    checklist = _rows(
        db.execute(
            "SELECT id,feature_id,body,checked,created_at,updated_at FROM solution_checklist_items WHERE feature_id=? ORDER BY created_at,rowid",
            (feature_id,),
        ).fetchall()
    )

    source_material = {
        "capture": capture,
        "problem": problem,
        "feature": feature,
        "completion": completion,
        "decisions": decisions,
        "conflict_reports": conflict_reports,
        "conflict_addresses": conflict_addresses,
        "completion_decisions": completion_decisions,
        "reviews": reviews,
        "progress": progress,
        "checklist": checklist,
    }
    source_hash = _digest(source_material)
    claims: list[dict[str, object]] = []
    evidence: dict[str, dict[str, object]] = {}

    def add_evidence(source_type: str, source_id: str, field_name: str, excerpt: object, live_type: str = "") -> str:
        key = f"evidence:{source_type}:{source_id}:{field_name}"
        text = _concise(excerpt, 800)
        evidence[key] = {
            "key": key,
            "source_type": source_type,
            "source_id": str(source_id),
            "field_name": field_name,
            "excerpt": text,
            "source_hash": _digest(text),
            "live_record": {"entity_type": live_type, "entity_id": str(source_id)} if live_type else None,
        }
        return key

    def add_claim(
        key: str,
        section: str,
        subject_type: str,
        subject_id: str,
        classification: str,
        text: object,
        evidence_keys: list[str],
        *,
        material: bool = False,
        confidence: str | None = None,
    ) -> str:
        claims.append(
            {
                "claim_key": key,
                "section": section,
                "subject_type": subject_type,
                "subject_id": str(subject_id),
                "classification": classification,
                "confidence": confidence,
                "material": material,
                "text": _concise(text),
                "evidence_keys": evidence_keys,
            }
        )
        return key

    capture_evidence = add_evidence(
        "capture",
        capture.get("id", "missing"),
        "text",
        capture.get("text") or ABSENT_REASON,
        "captures" if capture else "",
    )
    capture_claim = add_claim(
        "stage:capture",
        "stage",
        "capture",
        capture.get("id", "missing"),
        "observed",
        capture.get("text") or ABSENT_REASON,
        [capture_evidence],
        material=True,
    )

    desired_outcome = _section(
        str(problem.get("detail", "")), "desired outcome", "intended outcome", "목표", "의도한 결과"
    )
    problem_text = str(problem.get("statement") or ABSENT_REASON)
    if desired_outcome:
        problem_text += f"\nDesired outcome: {desired_outcome}"
    problem_evidence = add_evidence(
        "problem", problem.get("id", "missing"), "statement/detail", problem_text, "problems" if problem else ""
    )
    problem_claim = add_claim(
        "stage:problem",
        "stage",
        "problem",
        problem.get("id", "missing"),
        "observed",
        problem_text,
        [problem_evidence],
        material=True,
    )

    solution_text = f"{feature.get('title', '')}: {feature.get('outcome', '')}".strip(": ")
    solution_evidence = add_evidence("solution", feature_id, "title/outcome", solution_text, "features")
    solution_claim = add_claim(
        "stage:solution", "stage", "solution", feature_id, "decided", solution_text, [solution_evidence], material=True
    )

    latest_completion_decision = completion_decisions[-1] if completion_decisions else {}
    complete_text = latest_completion_decision.get("reason") or completion.get("report") or ABSENT_REASON
    complete_id = completion.get("id") or latest_completion_decision.get("id") or feature_id
    complete_evidence = add_evidence(
        "completion_decision" if latest_completion_decision else "completion",
        complete_id,
        "reason" if latest_completion_decision else "report",
        complete_text,
    )
    complete_claim = add_claim(
        "stage:complete", "stage", "complete", complete_id, "decided", complete_text, [complete_evidence], material=True
    )

    promotion_text = f"Original feedback was refined into the Problem: {problem.get('statement') or ABSENT_REASON}"
    promotion_claim = add_claim(
        "transition:capture-problem",
        "transition",
        "problem",
        problem.get("id", "missing"),
        "decided",
        promotion_text,
        [capture_evidence, problem_evidence],
        material=True,
    )
    created_event = next((item for item in decisions if item.get("event_type") == "created"), {})
    created_reason = str(created_event.get("reason") or "").strip()
    solution_change = (
        created_reason
        or f"The Problem was shaped into the Solution '{feature.get('title') or ABSENT_REASON}' with intended outcome: "
        f"{feature.get('outcome') or ABSENT_REASON}"
    )
    created_evidence = add_evidence(
        "solution_decision", created_event.get("id", feature_id), "reason", created_reason or ABSENT_REASON
    )
    solution_transition_claim = add_claim(
        "transition:problem-solution",
        "transition",
        "solution",
        feature_id,
        "decided",
        solution_change,
        [problem_evidence, solution_evidence, created_evidence],
        material=True,
    )
    completion_transition_claim = add_claim(
        "transition:solution-complete",
        "transition",
        "complete",
        complete_id,
        "decided",
        complete_text,
        [solution_evidence, complete_evidence],
        material=True,
    )

    decision_changes: list[dict[str, object]] = []
    for event in decisions:
        if event.get("event_type") in {"created", "approved", "completed"}:
            continue
        before = json.loads(event.get("before_json") or "{}")
        after = json.loads(event.get("after_json") or "{}")
        change_text = event.get("reason") or ABSENT_REASON
        if before or after:
            change_text = f"{change_text} Before: {_concise(before, 180)} After: {_concise(after, 180)}"
        event_evidence = add_evidence("solution_decision", event["id"], "decision", change_text)
        claim_key = add_claim(
            f"decision:{event['id']}",
            "decision_change",
            "solution",
            feature_id,
            "decided",
            change_text,
            [event_evidence],
            material=True,
        )
        decision_changes.append(
            {"claim_key": claim_key, "event_type": event.get("event_type"), "created_at": event.get("created_at")}
        )

    address_by_report = {str(item.get("conflict_report_id")): item for item in conflict_addresses}
    conflicts: list[dict[str, object]] = []
    for report in conflict_reports:
        if report.get("state") == "clear" and not any(item.get("state") == "conflicted" for item in conflict_reports):
            continue
        address = address_by_report.get(str(report["id"]))
        if address and address.get("status") in {"detected", "addressed", "unaddressed", "unclear"}:
            status = str(address["status"])
        elif report.get("state") == "conflicted":
            status = "unaddressed"
        else:
            status = "unclear"
        text = str(report.get("citation") or ABSENT_REASON)
        if address:
            text = f"{text} Address: {address.get('summary') or ABSENT_REASON}"
        report_evidence = add_evidence(
            "conflict_report", report["id"], "citation", report.get("citation") or ABSENT_REASON
        )
        evidence_keys = [report_evidence]
        if address:
            evidence_keys.append(
                add_evidence(
                    str(address.get("evidence_source_type") or "conflict_address"),
                    str(address.get("evidence_source_id") or address["id"]),
                    "address",
                    address.get("summary") or ABSENT_REASON,
                )
            )
        claim_key = add_claim(
            f"conflict:{report['id']}",
            "conflict",
            "solution",
            feature_id,
            "decided" if status == "addressed" else "observed",
            text,
            evidence_keys,
            material=status != "unclear",
        )
        conflicts.append(
            {
                "claim_key": claim_key,
                "report_id": report["id"],
                "status": status,
                "basis": address.get("basis") if address else None,
                "disposition": address.get("disposition") if address else None,
                "created_at": report.get("created_at"),
            }
        )

    completion_evidence_items: list[dict[str, object]] = []
    for name, value in (("evidence", completion.get("evidence")), ("report", completion.get("report"))):
        if not value:
            continue
        ev = add_evidence("completion", completion.get("id", feature_id), name, value)
        key = add_claim(f"completion:{name}", "completion_evidence", "complete", complete_id, "observed", value, [ev])
        completion_evidence_items.append({"claim_key": key, "kind": name})
    for item in progress:
        text = item.get("body") or item.get("image_summary")
        if not text:
            continue
        ev = add_evidence("work_log", item["id"], "body/image_summary", text)
        key = add_claim(f"worklog:{item['id']}", "completion_evidence", "solution", feature_id, "observed", text, [ev])
        completion_evidence_items.append({"claim_key": key, "kind": "work_log"})
    for item in checklist:
        text = f"{'Met' if item.get('checked') else 'Not met'}: {item.get('body', '')}"
        ev = add_evidence("checklist", item["id"], "body/checked", text)
        key = add_claim(
            f"checklist:{item['id']}", "completion_evidence", "solution", feature_id, "observed", text, [ev]
        )
        completion_evidence_items.append({"claim_key": key, "kind": "validation"})
    for review in reviews[-1:]:
        review_text = review.get("report_json") or ABSENT_REASON
        ev = add_evidence("completion_review", review["id"], "report_json", review_text)
        key = add_claim(
            f"review:{review['id']}",
            "completion_evidence",
            "complete",
            complete_id,
            "inferred",
            review_text,
            [ev],
            confidence="medium",
        )
        completion_evidence_items.append({"claim_key": key, "kind": "ai_completion_review"})

    conflict_priority = {"unaddressed": 4, "addressed": 3, "detected": 2, "unclear": 1}
    material_conflict = max(
        conflicts,
        key=lambda item: (
            conflict_priority.get(str(item.get("status")), 0),
            int(item.get("disposition") in {"modified", "superseded", "rejected"}),
        ),
        default=None,
    )
    document: dict[str, object] = {
        "schema_version": LINEAGE_SCHEMA_VERSION,
        "source_hash": source_hash,
        "detail": {
            "title": feature.get("title", ""),
            "outcome": feature.get("outcome", ""),
            "non_goals": feature.get("non_goals", ""),
            "validation_criteria": feature.get("validation_criteria", ""),
        },
        "lineage": {
            "stages": [
                {
                    "kind": "capture",
                    "record_type": "captures",
                    "record_id": capture.get("id"),
                    "title": "Capture",
                    "claim_key": capture_claim,
                    "occurred_at": capture.get("created_at"),
                    "live_available": bool(capture),
                },
                {
                    "kind": "problem",
                    "record_type": "problems",
                    "record_id": problem.get("id"),
                    "title": "Problem",
                    "claim_key": problem_claim,
                    "occurred_at": problem.get("created_at"),
                    "live_available": bool(problem),
                },
                {
                    "kind": "solution",
                    "record_type": "features",
                    "record_id": feature_id,
                    "title": "Solution",
                    "claim_key": solution_claim,
                    "occurred_at": feature.get("created_at"),
                    "live_available": True,
                },
                {
                    "kind": "complete",
                    "record_type": "",
                    "record_id": complete_id,
                    "title": "Complete",
                    "claim_key": complete_claim,
                    "occurred_at": latest_completion_decision.get("created_at") or completion.get("created_at"),
                    "live_available": bool(completion or completion_decisions),
                },
            ],
            "transitions": [
                {"from": "capture", "to": "problem", "claim_key": promotion_claim, "context_kind": "recorded_change"},
                {
                    "from": "problem",
                    "to": "solution",
                    "claim_key": solution_transition_claim,
                    "context_kind": "decision_basis" if created_reason else "recorded_change",
                    "material_conflict": material_conflict,
                },
                {
                    "from": "solution",
                    "to": "complete",
                    "claim_key": completion_transition_claim,
                    "context_kind": "decision_basis" if complete_text != ABSENT_REASON else "recorded_change",
                },
            ],
        },
        "claims": claims,
        "evidence": list(evidence.values()),
        "decision_changes": decision_changes,
        "conflicts": conflicts,
        "completion_evidence": completion_evidence_items,
    }
    return document, source_hash


def report_context(lineages: list[dict[str, object]]) -> str:
    payload: list[dict[str, object]] = []
    for lineage in lineages:
        claims = lineage.get("claims", {})
        if isinstance(claims, list):
            claims = {str(item.get("id") or item.get("claim_key")): item for item in claims if isinstance(item, dict)}
        evidence = lineage.get("evidence", {})
        if isinstance(evidence, list):
            evidence = {str(item.get("id") or item.get("key")): item for item in evidence if isinstance(item, dict)}
        referenced = {
            evidence_id: evidence[evidence_id]
            for claim in claims.values()
            for evidence_id in claim.get("evidence_ids", [])
            if evidence_id in evidence
        }
        payload.append(
            {
                "snapshot_id": lineage.get("snapshot_id"),
                "version": lineage.get("version"),
                "detail": lineage.get("detail"),
                "lineage": lineage.get("lineage"),
                "claims": list(claims.values()),
                "decision_changes": lineage.get("decision_changes", []),
                "conflicts": lineage.get("conflicts", []),
                "completion_evidence": lineage.get("completion_evidence", []),
                "referenced_evidence": list(referenced.values()),
            }
        )
    return json.dumps({"lineage_snapshots": payload}, ensure_ascii=False, separators=(",", ":"))


def readable_report_context(lineages: list[dict[str, object]]) -> str:
    """Project Lineage into a final-report context without database identifiers."""
    payload: list[dict[str, object]] = []
    fixed_labels = {
        "capture": "Original capture",
        "problem": "Problem record",
        "solution": "Solution record",
        "completion": "Completion record",
        "completion_decision": "Completion decision",
    }
    numbered_labels = {
        "work_log": "Work log",
        "checklist": "Validation criterion",
        "completion_review": "Completion review",
        "solution_decision": "Decision record",
        "conflict_report": "Conflict review",
    }
    for lineage in lineages:
        claims = lineage.get("claims", {})
        if isinstance(claims, list):
            claims = {str(item.get("id") or item.get("claim_key")): item for item in claims if isinstance(item, dict)}
        evidence = lineage.get("evidence", {})
        if isinstance(evidence, list):
            evidence = {str(item.get("id") or item.get("key")): item for item in evidence if isinstance(item, dict)}
        claim_keys = {str(identifier): f"Claim {index}" for index, identifier in enumerate(claims, 1)}
        labels_by_evidence: dict[str, str] = {}
        labels_by_source: dict[tuple[str, str, str, str], str] = {}
        label_counts: dict[str, int] = {}
        readable_evidence: list[dict[str, object]] = []
        for claim in claims.values():
            for evidence_id in claim.get("evidence_ids", []):
                identifier = str(evidence_id)
                item = evidence.get(identifier)
                if not item:
                    continue
                source_type = str(item.get("source_type") or "evidence")
                source_key = (
                    source_type,
                    str(item.get("source_id") or ""),
                    str(item.get("field_name") or ""),
                    str(item.get("source_hash") or ""),
                )
                label = labels_by_source.get(source_key)
                if not label:
                    label_counts[source_type] = label_counts.get(source_type, 0) + 1
                    base = (
                        fixed_labels.get(source_type)
                        or numbered_labels.get(source_type)
                        or source_type.replace("_", " ").title()
                    )
                    count = label_counts[source_type]
                    label = base if source_type in fixed_labels and count == 1 else f"{base} {count}"
                    labels_by_source[source_key] = label
                    readable_evidence.append(
                        {
                            "label": label,
                            "source_type": source_type,
                            "field_name": item.get("field_name"),
                            "excerpt": item.get("excerpt"),
                            "captured_at": item.get("captured_at"),
                        }
                    )
                labels_by_evidence[identifier] = label

        def safe(value: object) -> object:
            if isinstance(value, list):
                return [safe(item) for item in value]
            if not isinstance(value, dict):
                return value
            result: dict[str, object] = {}
            for key, item in value.items():
                if key == "claim_id":
                    result["claim_key"] = claim_keys.get(str(item), "recorded-claim")
                elif key == "evidence_ids":
                    result["evidence_labels"] = [
                        labels_by_evidence[str(identifier)]
                        for identifier in item
                        if str(identifier) in labels_by_evidence
                    ]
                elif key == "id" or key.endswith("_id") or key in {"source_hash", "revisions", "current_revision_id"}:
                    continue
                else:
                    result[key] = safe(item)
            return result

        readable_claims = []
        for identifier, claim in claims.items():
            readable_claims.append(
                {
                    "claim_key": claim_keys[str(identifier)],
                    "section": claim.get("section"),
                    "classification": claim.get("classification"),
                    "confidence": claim.get("confidence"),
                    "text": claim.get("text"),
                    "evidence_labels": [
                        labels_by_evidence[str(evidence_id)]
                        for evidence_id in claim.get("evidence_ids", [])
                        if str(evidence_id) in labels_by_evidence
                    ],
                }
            )
        detail = lineage.get("detail", {})
        payload.append(
            {
                "detail": {
                    key: detail.get(key)
                    for key in ("title", "outcome", "non_goals", "validation_criteria")
                    if isinstance(detail, dict) and key in detail
                },
                "lineage": safe(lineage.get("lineage", {})),
                "claims": readable_claims,
                "decision_changes": safe(lineage.get("decision_changes", [])),
                "conflicts": safe(lineage.get("conflicts", [])),
                "completion_evidence": safe(lineage.get("completion_evidence", [])),
                "referenced_evidence": readable_evidence,
            }
        )
    return json.dumps({"lineage_snapshots": payload}, ensure_ascii=False, separators=(",", ":"))


def render_lineage_markdown(lineage: dict[str, object]) -> list[str]:
    claims = lineage.get("claims", {})
    if isinstance(claims, list):
        claims = {str(item.get("id") or item.get("claim_key")): item for item in claims if isinstance(item, dict)}

    def claim_text(reference: str | None) -> str:
        item = claims.get(str(reference), {})
        label = str(item.get("classification", "observed")).title()
        confidence = f" · {str(item.get('confidence')).title()} confidence" if item.get("confidence") else ""
        return f"[{label}{confidence}] {item.get('text', ABSENT_REASON)}"

    lines = ["## Detail"]
    detail = lineage.get("detail", {})
    lines.extend(
        [
            f"- **Solution**: {detail.get('title') or ABSENT_REASON}",
            f"- **Intended outcome**: {detail.get('outcome') or ABSENT_REASON}",
            f"- **Non-goals**: {detail.get('non_goals') or 'None recorded.'}",
            "- **Validation Criteria**:",
            str(detail.get("validation_criteria") or ABSENT_REASON),
            "",
            "## Lineage",
        ]
    )
    for index, stage in enumerate(lineage.get("lineage", {}).get("stages", [])):
        lines.extend([f"### {stage.get('title')}", claim_text(stage.get("claim_id") or stage.get("claim_key"))])
        transitions = lineage.get("lineage", {}).get("transitions", [])
        if index < len(transitions):
            transition = transitions[index]
            lines.append(f"→ {claim_text(transition.get('claim_id') or transition.get('claim_key'))}")
            if transition.get("material_conflict"):
                conflict = transition["material_conflict"]
                lines.append(
                    f"  - Conflict: {conflict.get('status')} · {conflict.get('disposition') or 'disposition not recorded'}"
                )
    lines.extend(["", "## Decision Changes"])
    for item in lineage.get("decision_changes", []):
        lines.append(f"- {claim_text(item.get('claim_id') or item.get('claim_key'))}")
    if not lineage.get("decision_changes"):
        lines.append(f"- {ABSENT_REASON}")
    lines.extend(["", "## Conflicts & Addresses"])
    for item in lineage.get("conflicts", []):
        lines.append(
            f"- **{str(item.get('status', 'unclear')).title()}** · basis: {item.get('basis') or 'Not recorded'} · "
            f"disposition: {item.get('disposition') or 'Not recorded'} — {claim_text(item.get('claim_id') or item.get('claim_key'))}"
        )
    if not lineage.get("conflicts"):
        lines.append("- No material conflict was recorded.")
    lines.extend(["", "## Completion Evidence"])
    for item in lineage.get("completion_evidence", []):
        lines.append(f"- {claim_text(item.get('claim_id') or item.get('claim_key'))}")
    if not lineage.get("completion_evidence"):
        lines.append("- No recorded evidence.")
    return lines
