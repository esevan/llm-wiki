from __future__ import annotations

import ast
import sqlite3
import uuid
import json
import re
import base64
from datetime import date
from pathlib import Path

from llm_wiki.services.localization import LocalizedContentStore
from llm_wiki.services.lineage import LINEAGE_SCHEMA_VERSION, build_lineage_document, render_lineage_markdown


class WorkflowError(ValueError):
    pass


# Workflow Transitions: explicit, menu-only routes with required input forms.
# Each transition declares the source entity type, the human-readable label, and
# the required input fields so the frontend can render a form without hard-coding.
TRANSITIONS: list[dict[str, object]] = [
    {
        "id": "capture_to_problem",
        "label": "Create Problem manually",
        "submit_label": "Create",
        "source_type": "captures",
        "description": "Create the next Problem directly when AI assistance is unavailable.",
        "fields": [
            {"name": "statement", "label": "Problem statement", "type": "text", "required": True, "placeholder": "What needs to change?"},
            {"name": "detail", "label": "Context", "type": "textarea", "required": False, "placeholder": "Evidence, impact, constraints, or open questions"},
        ],
    },
    {
        "id": "problem_to_solution",
        "label": "Create Solution manually",
        "submit_label": "Create",
        "source_type": "problems",
        "description": "Create a Solution directly from this approved Problem without using AI.",
        "fields": [
            {"name": "title", "label": "Solution name", "type": "text", "required": True, "placeholder": "A short, outcome-focused name"},
            {"name": "outcome", "label": "Intended outcome", "type": "textarea", "required": True, "placeholder": "What will be true when this works?"},
            {"name": "non_goals", "label": "Non-goals", "type": "textarea", "required": False, "placeholder": "What is intentionally out of scope?"},
            {"name": "validation_criteria", "label": "Validation criteria", "type": "textarea", "required": True, "placeholder": "- [ ] Observable result", "help": "Add at least one checklist item using - [ ]."},
        ],
    },
    {
        "id": "solution_to_approved",
        "label": "Start manually",
        "submit_label": "Start work",
        "source_type": "features",
        "description": "Move this Solution to In progress without an AI conflict review.",
        "fields": [
            {"name": "approval_path", "label": "Conflict check", "type": "select", "required": True, "options": [
                {"value": "checked", "label": "Already checked"},
                {"value": "skip", "label": "Skip with a reason"},
            ]},
            {"name": "citation", "label": "Review basis", "type": "textarea", "required_when": {"approval_path": "checked"}, "visible_when": {"approval_path": "checked"}, "placeholder": "Vault path, Workbench item, or review note"},
            {"name": "skip_reason", "label": "Skip reason", "type": "textarea", "required_when": {"approval_path": "skip"}, "visible_when": {"approval_path": "skip"}, "placeholder": "Why is it safe to start without a conflict check?"},
        ],
    },
    {
        "id": "solution_to_completed",
        "label": "Complete manually",
        "submit_label": "Complete",
        "source_type": "features",
        "description": "Complete and archive this work directly without an AI completion review.",
        "fields": [
            {"name": "evidence", "label": "Completion evidence", "type": "textarea", "required": True, "placeholder": "What proves the intended outcome was reached?"},
            {"name": "completion_path", "label": "Knowledge record", "type": "select", "required": True, "options": [
                {"value": "report", "label": "Add completion note"},
                {"value": "no_update", "label": "Skip note"},
            ]},
            {"name": "report", "label": "Completion note", "type": "textarea", "required_when": {"completion_path": "report"}, "visible_when": {"completion_path": "report"}, "placeholder": "Summarize what changed and what was learned"},
            {"name": "no_update_reason", "label": "No-update reason", "type": "textarea", "required_when": {"completion_path": "no_update"}, "visible_when": {"completion_path": "no_update"}, "placeholder": "Why is no reusable knowledge update needed?"},
            {"name": "reason", "label": "Decision note", "type": "textarea", "required": False, "placeholder": "Optional: why this Problem can close now"},
        ],
    },
]


def available_transitions(entity_type: str, entity: dict[str, object] | None = None) -> list[dict[str, object]]:
    """Return transitions available for this entity type, filtered by current state."""
    result: list[dict[str, object]] = []
    for transition in TRANSITIONS:
        if transition["source_type"] != entity_type:
            continue
        # Solution→Approved only applies to proposed (not yet approved) solutions.
        if transition["id"] == "solution_to_approved" and entity and entity.get("state") == "approved":
            continue
        # In progress→Completed only applies to approved (in progress) solutions.
        if transition["id"] == "solution_to_completed" and entity and entity.get("state") != "approved":
            continue
        # A Solution can be created only after the Problem approval gate.
        if transition["id"] == "problem_to_solution" and entity and entity.get("state") != "approved":
            continue
        result.append(transition)
    return result


class WorkflowEngine:
    """Human-operated workflow state. This service has no provider or vault dependency."""


    def __init__(self, db: sqlite3.Connection):
        self.db = db
        self._init_schema()
        self.localized = LocalizedContentStore(db)

    def _init_schema(self) -> None:
        self.db.executescript("""
        CREATE TABLE IF NOT EXISTS captures (
          id TEXT PRIMARY KEY, text TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS problems (
          id TEXT PRIMARY KEY, capture_id TEXT UNIQUE, statement TEXT NOT NULL,
          detail TEXT NOT NULL DEFAULT '', state TEXT NOT NULL DEFAULT 'draft', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS questions (
          id TEXT PRIMARY KEY, capture_id TEXT, question TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'open', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS features (
          id TEXT PRIMARY KEY, problem_id TEXT NOT NULL, title TEXT NOT NULL,
          outcome TEXT NOT NULL, non_goals TEXT NOT NULL DEFAULT '', conflict_state TEXT NOT NULL DEFAULT 'unknown',
          validation_criteria TEXT NOT NULL DEFAULT '', state TEXT NOT NULL DEFAULT 'proposed', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS solution_progress_entries (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, body TEXT NOT NULL DEFAULT '',
          image_data TEXT NOT NULL DEFAULT '', image_media_type TEXT NOT NULL DEFAULT '',
          image_summary TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS solution_progress_comments (
          id TEXT PRIMARY KEY, entry_id TEXT NOT NULL, body TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS solution_checklist_items (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, body TEXT NOT NULL,
          checked INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS approvals (
          id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, action TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS conflict_reports (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, state TEXT NOT NULL, citation TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS conflict_review_runs (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, status TEXT NOT NULL, query TEXT NOT NULL,
          candidates_json TEXT NOT NULL DEFAULT '[]', report_json TEXT NOT NULL DEFAULT '{}',
          error TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS solution_decision_events (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, event_type TEXT NOT NULL,
          before_json TEXT NOT NULL DEFAULT '{}', after_json TEXT NOT NULL DEFAULT '{}',
          reason TEXT NOT NULL DEFAULT '', provenance TEXT NOT NULL DEFAULT 'decided',
          source_type TEXT NOT NULL DEFAULT '', source_id TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS conflict_addresses (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, conflict_report_id TEXT NOT NULL,
          status TEXT NOT NULL, basis TEXT NOT NULL, disposition TEXT,
          summary TEXT NOT NULL DEFAULT '', evidence_source_type TEXT NOT NULL DEFAULT '',
          evidence_source_id TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS completions (
          id TEXT PRIMARY KEY, feature_id TEXT UNIQUE NOT NULL, evidence TEXT NOT NULL, report TEXT NOT NULL,
          knowledge_status TEXT NOT NULL DEFAULT 'pending', no_update_reason TEXT NOT NULL DEFAULT '', state TEXT NOT NULL DEFAULT 'draft',
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS completion_reviews (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'ready',
          report_json TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS follow_up_links (
          problem_id TEXT PRIMARY KEY, source_feature_id TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS problem_completion_decisions (
          id TEXT PRIMARY KEY, problem_id TEXT NOT NULL, review_id TEXT NOT NULL DEFAULT '',
          reason TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS completion_playbooks (
          problem_id TEXT PRIMARY KEY, path TEXT NOT NULL, source_hash TEXT NOT NULL,
          lineage_snapshot_id TEXT NOT NULL DEFAULT '', lineage_version INTEGER NOT NULL DEFAULT 0,
          lineage_schema_version INTEGER NOT NULL DEFAULT 0, report_input_hash TEXT NOT NULL DEFAULT '',
          report_generation_status TEXT NOT NULL DEFAULT 'deterministic_fallback',
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS lineage_snapshots (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, version INTEGER NOT NULL,
          schema_version INTEGER NOT NULL, source_hash TEXT NOT NULL, status TEXT NOT NULL,
          document_json TEXT NOT NULL DEFAULT '{}', inference_error TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          UNIQUE(feature_id,version)
        );
        CREATE TABLE IF NOT EXISTS lineage_claims (
          id TEXT PRIMARY KEY, snapshot_id TEXT NOT NULL, claim_key TEXT NOT NULL,
          section TEXT NOT NULL, subject_type TEXT NOT NULL, subject_id TEXT NOT NULL,
          classification TEXT NOT NULL, confidence TEXT, material INTEGER NOT NULL DEFAULT 0,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          UNIQUE(snapshot_id,claim_key)
        );
        CREATE TABLE IF NOT EXISTS lineage_evidence (
          id TEXT PRIMARY KEY, claim_id TEXT NOT NULL, source_type TEXT NOT NULL,
          source_id TEXT NOT NULL, field_name TEXT NOT NULL, excerpt TEXT NOT NULL,
          source_hash TEXT NOT NULL, live_entity_type TEXT NOT NULL DEFAULT '',
          captured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS lineage_revisions (
          id TEXT PRIMARY KEY, claim_id TEXT NOT NULL, supersedes_id TEXT,
          author_type TEXT NOT NULL, text TEXT NOT NULL, reason TEXT NOT NULL DEFAULT '',
          is_current INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS patch_proposals (
          id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, path TEXT NOT NULL, operation TEXT NOT NULL, heading TEXT NOT NULL,
          content TEXT NOT NULL, base_hash TEXT NOT NULL, before_text TEXT NOT NULL, proposed_text TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'proposed', reverse_text TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS compass_goals (
          id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS importance_assessments (
          id TEXT PRIMARY KEY, problem_id TEXT UNIQUE NOT NULL, alignment INTEGER NOT NULL, impact INTEGER NOT NULL, urgency INTEGER NOT NULL, leverage INTEGER NOT NULL,
          evidence TEXT NOT NULL, importance INTEGER NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS score_events (
          id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, goal_id TEXT, points REAL NOT NULL, event_type TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS score_periods (
          period TEXT NOT NULL, goal_id TEXT, points REAL NOT NULL, PRIMARY KEY(period, goal_id)
        );
        CREATE TABLE IF NOT EXISTS mirror_files (
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, path TEXT NOT NULL, source_hash TEXT NOT NULL,
          PRIMARY KEY(entity_type, entity_id)
        );
        CREATE TABLE IF NOT EXISTS deleted_entities (
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, deleted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          PRIMARY KEY(entity_type, entity_id)
        );
        CREATE TABLE IF NOT EXISTS ai_runs (
          id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, kind TEXT NOT NULL,
          input_text TEXT NOT NULL, output_text TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS workbench_priorities (
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, category TEXT NOT NULL, attention_rank INTEGER NOT NULL,
          rationale TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          PRIMARY KEY(entity_type, entity_id)
        );
        CREATE TABLE IF NOT EXISTS workbench_priority_overrides (
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, manual_priority INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY(entity_type, entity_id)
        );
        CREATE TABLE IF NOT EXISTS workbench_category_overrides (
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, category TEXT NOT NULL,
          PRIMARY KEY(entity_type, entity_id)
        );
        """)
        problem_columns = {row[1] for row in self.db.execute("PRAGMA table_info(problems)")}
        if "detail" not in problem_columns:
            self.db.execute("ALTER TABLE problems ADD COLUMN detail TEXT NOT NULL DEFAULT ''")
        feature_columns = {row[1] for row in self.db.execute("PRAGMA table_info(features)")}
        if "validation_criteria" not in feature_columns:
            self.db.execute("ALTER TABLE features ADD COLUMN validation_criteria TEXT NOT NULL DEFAULT ''")
        playbook_columns = {row[1] for row in self.db.execute("PRAGMA table_info(completion_playbooks)")}
        for name, declaration in {
            "lineage_snapshot_id": "TEXT NOT NULL DEFAULT ''",
            "lineage_version": "INTEGER NOT NULL DEFAULT 0",
            "lineage_schema_version": "INTEGER NOT NULL DEFAULT 0",
            "report_input_hash": "TEXT NOT NULL DEFAULT ''",
            "report_generation_status": "TEXT NOT NULL DEFAULT 'deterministic_fallback'",
        }.items():
            if name not in playbook_columns:
                self.db.execute(f"ALTER TABLE completion_playbooks ADD COLUMN {name} {declaration}")
        self.db.execute("CREATE INDEX IF NOT EXISTS idx_lineage_snapshots_feature ON lineage_snapshots(feature_id,version DESC)")
        self.db.execute("CREATE INDEX IF NOT EXISTS idx_lineage_claims_snapshot ON lineage_claims(snapshot_id)")
        self.db.execute("CREATE INDEX IF NOT EXISTS idx_lineage_evidence_claim ON lineage_evidence(claim_id)")
        self.db.execute("CREATE INDEX IF NOT EXISTS idx_lineage_revisions_claim ON lineage_revisions(claim_id,is_current)")
        self.db.commit()

    def capture(self, text: str) -> str:
        capture_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO captures(id, text) VALUES (?, ?)", (capture_id, text))
        self.db.commit()
        return capture_id

    def promote_capture(self, capture_id: str, statement: str | None = None, detail: str = "", localized_versions: dict[str, dict[str, str]] | None = None) -> dict[str, str]:
        capture = self.db.execute("SELECT text FROM captures WHERE id=?", (capture_id,)).fetchone()
        if not capture:
            raise WorkflowError("Capture not found")
        existing = self.db.execute("SELECT * FROM problems WHERE capture_id=?", (capture_id,)).fetchone()
        if existing:
            # Promotion is idempotent: repair any older record that predates
            # the Inbox-hide rule instead of leaving the Capture in both lanes.
            self.db.execute("INSERT OR REPLACE INTO deleted_entities(entity_type,entity_id) VALUES ('captures', ?)", (capture_id,))
            self.db.commit()
            return dict(existing)
        problem_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO problems(id, capture_id, statement, detail) VALUES (?, ?, ?, ?)", (problem_id, capture_id, statement or capture[0], detail))
        if localized_versions:
            problem_versions = {
                locale: {
                    "statement": fields.get("statement", fields.get("title", "")),
                    "detail": fields.get("detail", ""),
                }
                for locale, fields in localized_versions.items()
            }
            if set(problem_versions) == {"ko", "en"}:
                self.localized.save_bilingual("problems", problem_id, problem_versions)
            else:
                self.localized.save_versions("problems", problem_id, problem_versions, complete=True)
        self._inherit_workbench_category("captures", capture_id, "problems", problem_id, str(capture[0]))
        # A promoted capture stays linked for audit/history but leaves the active inbox.
        self.db.execute("INSERT OR REPLACE INTO deleted_entities(entity_type,entity_id) VALUES ('captures', ?)", (capture_id,))
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM problems WHERE id=?", (problem_id,)).fetchone())

    def approve_problem(self, problem_id: str) -> None:
        if not self.db.execute("SELECT 1 FROM problems WHERE id=?", (problem_id,)).fetchone():
            raise WorkflowError("Problem not found")
        self.db.execute("UPDATE problems SET state='approved' WHERE id=?", (problem_id,))
        self._approval("problem", problem_id, "approved")
        importance = self.db.execute("SELECT importance FROM importance_assessments WHERE problem_id=?", (problem_id,)).fetchone()
        if importance:
            self.award("problem", problem_id, float(importance[0]) * .10, "problem_approved")

    def _record_solution_decision(
        self,
        feature_id: str,
        event_type: str,
        before: dict[str, object],
        after: dict[str, object],
        reason: str = "",
        source_type: str = "human_action",
        source_id: str = "",
        provenance: str = "decided",
    ) -> str:
        event_id = str(uuid.uuid4())
        self.db.execute(
            """INSERT INTO solution_decision_events(
                 id,feature_id,event_type,before_json,after_json,reason,provenance,source_type,source_id
               ) VALUES (?,?,?,?,?,?,?,?,?)""",
            (
                event_id,
                feature_id,
                event_type,
                json.dumps(before, ensure_ascii=False, sort_keys=True),
                json.dumps(after, ensure_ascii=False, sort_keys=True),
                reason.strip(),
                provenance,
                source_type,
                source_id,
            ),
        )
        return event_id

    def create_feature(self, problem_id: str, title: str, outcome: str, non_goals: str = "", validation_criteria: str = "", localized_versions: dict[str, dict[str, str]] | None = None) -> dict[str, str]:
        problem = self.db.execute("SELECT state, statement FROM problems WHERE id=?", (problem_id,)).fetchone()
        if not problem:
            raise WorkflowError("Problem not found")
        if problem[0] != "approved":
            raise WorkflowError("Features require an approved problem")
        if not validation_criteria.strip():
            raise WorkflowError("A Solution requires at least one Validation Criteria bullet")
        feature_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO features(id,problem_id,title,outcome,non_goals,validation_criteria) VALUES (?,?,?,?,?,?)", (feature_id, problem_id, title, outcome, non_goals, validation_criteria))
        if localized_versions:
            if set(localized_versions) == {"ko", "en"}:
                self.localized.save_bilingual("features", feature_id, localized_versions)
            else:
                self.localized.save_versions("features", feature_id, localized_versions, complete=True)
        self._record_solution_decision(
            feature_id,
            "created",
            {},
            {"title": title, "outcome": outcome, "non_goals": non_goals, "validation_criteria": validation_criteria},
            "",
            "human_action",
            feature_id,
        )
        self._inherit_workbench_category("problems", problem_id, "features", feature_id, str(problem[1]))
        self.db.commit()
        self.seed_solution_checklist(feature_id, validation_criteria)
        return dict(self.db.execute("SELECT * FROM features WHERE id=?", (feature_id,)).fetchone())

    def record_conflict_evaluation(self, feature_id: str, state: str, citation: str, commit: bool = True) -> str:
        if state not in {"unknown", "conflicted", "clear"}:
            raise WorkflowError("Invalid conflict state")
        if not self.db.execute("SELECT 1 FROM features WHERE id=?", (feature_id,)).fetchone():
            raise WorkflowError("Feature not found")
        if state == "clear" and not citation.strip():
            raise WorkflowError("A clear evaluation requires a cited current-context basis")
        self.db.execute("UPDATE features SET conflict_state=? WHERE id=?", (state, feature_id))
        report_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO conflict_reports(id,feature_id,state,citation) VALUES (?,?,?,?)", (report_id, feature_id, state, citation))
        if commit:
            self.db.commit()
        return report_id

    def record_conflict_address(
        self,
        feature_id: str,
        conflict_report_id: str,
        status: str,
        basis: str,
        disposition: str | None,
        summary: str,
        evidence_source_type: str,
        evidence_source_id: str,
        commit: bool = True,
    ) -> dict[str, str]:
        if status not in {"detected", "addressed", "unaddressed", "unclear"}:
            raise WorkflowError("Invalid conflict address status")
        if basis not in {"explicit_decision", "implementation_evidence", "ai_inferred"}:
            raise WorkflowError("Invalid conflict address basis")
        if disposition not in {None, "preserved", "modified", "superseded", "rejected"}:
            raise WorkflowError("Invalid conflict requirement disposition")
        report = self.db.execute(
            "SELECT 1 FROM conflict_reports WHERE id=? AND feature_id=?", (conflict_report_id, feature_id)
        ).fetchone()
        if not report:
            raise WorkflowError("Conflict report not found")
        if status == "addressed":
            if basis not in {"explicit_decision", "implementation_evidence"}:
                raise WorkflowError("AI inference cannot mark a conflict Addressed")
            if not disposition or not evidence_source_type.strip() or not evidence_source_id.strip():
                raise WorkflowError("Addressed conflicts require disposition and supporting evidence")
        address_id = str(uuid.uuid4())
        self.db.execute(
            """INSERT INTO conflict_addresses(
                 id,feature_id,conflict_report_id,status,basis,disposition,summary,evidence_source_type,evidence_source_id
               ) VALUES (?,?,?,?,?,?,?,?,?)""",
            (address_id, feature_id, conflict_report_id, status, basis, disposition, summary.strip(), evidence_source_type.strip(), evidence_source_id.strip()),
        )
        self._record_solution_decision(
            feature_id,
            "conflict_addressed" if status == "addressed" else "conflict_status_recorded",
            {},
            {"status": status, "basis": basis, "disposition": disposition},
            summary,
            "conflict_address",
            address_id,
        )
        if commit:
            self.db.commit()
        return dict(self.db.execute("SELECT * FROM conflict_addresses WHERE id=?", (address_id,)).fetchone())

    def start_conflict_review(self, feature_id: str, query: str) -> str:
        if not self.db.execute("SELECT 1 FROM features WHERE id=?", (feature_id,)).fetchone():
            raise WorkflowError("Solution not found")
        run_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO conflict_review_runs(id,feature_id,status,query) VALUES (?,?,?,?)", (run_id, feature_id, "running", query))
        self.db.commit()
        return run_id

    def finish_conflict_review(self, run_id: str, candidates: object, report: object, error: str = "") -> None:
        status = "failed" if error else "ready"
        self.db.execute("UPDATE conflict_review_runs SET status=?,candidates_json=?,report_json=?,error=?,updated_at=CURRENT_TIMESTAMP WHERE id=?", (status, __import__("json").dumps(candidates), __import__("json").dumps(report), error, run_id))
        self.db.commit()

    def cancel_conflict_review(self, run_id: str, report: object) -> None:
        self.db.execute(
            "UPDATE conflict_review_runs SET status='cancelled',report_json=?,updated_at=CURRENT_TIMESTAMP WHERE id=?",
            (__import__("json").dumps(report), run_id),
        )
        self.db.commit()

    def cached_conflict_review(self, query: str) -> dict[str, object] | None:
        row = self.db.execute(
            "SELECT report_json FROM conflict_review_runs WHERE query=? AND status='ready' ORDER BY updated_at DESC,rowid DESC LIMIT 1",
            (query,),
        ).fetchone()
        if not row:
            return None
        try:
            report = __import__("json").loads(row[0])
        except (TypeError, ValueError):
            return None
        return report if isinstance(report, dict) else None

    def approve_feature(self, feature_id: str) -> None:
        feature = self.db.execute("SELECT conflict_state FROM features WHERE id=?", (feature_id,)).fetchone()
        if not feature:
            raise WorkflowError("Feature not found")
        if feature[0] != "clear":
            raise WorkflowError("Feature approval requires a clear current conflict report")
        self.db.execute("UPDATE features SET state='approved' WHERE id=?", (feature_id,))
        self._approval("feature", feature_id, "approved")
        self._record_solution_decision(feature_id, "approved", {"state": "proposed"}, {"state": "approved"}, "", "approval", feature_id)
        importance = self.db.execute("SELECT i.importance FROM importance_assessments i JOIN features f ON f.problem_id=i.problem_id WHERE f.id=?", (feature_id,)).fetchone()
        if importance:
            self.award("feature", feature_id, float(importance[0]) * .20, "feature_approved")

    def set_feature_stage(self, feature_id: str, state: str) -> None:
        if state not in {"proposed", "approved"}:
            raise WorkflowError("Unsupported Solution stage")
        row = self.db.execute("SELECT conflict_state FROM features WHERE id=?", (feature_id,)).fetchone()
        if not row:
            raise WorkflowError("Solution not found")
        if state == "approved" and row[0] != "clear":
            raise WorkflowError("Moving to in progress requires a clear conflict review")
        self.db.execute("UPDATE features SET state=? WHERE id=?", (state, feature_id))
        self._approval("feature", feature_id, f"moved_to_{state}")

    def apply_transition(self, transition_id: str, entity_type: str, entity_id: str, fields: dict[str, object]) -> dict[str, object]:
        """Apply a Workflow Transition with its required input form.

        This is the single entry point for menu-only transitions. Each transition
        validates its required fields and delegates to the appropriate workflow
        method, including the skip-conflict-check and complete-without-report paths.
        """
        fields = dict(fields)
        # Preserve compatibility with callers that predate the explicit path
        # selectors used by the manual-fallback form.
        if transition_id == "solution_to_approved" and not fields.get("approval_path"):
            fields["_legacy_conflict_state"] = fields.get("conflict_state", "unknown")
            fields["approval_path"] = "skip" if fields.get("skip_conflict_check") else "checked"
        if transition_id == "solution_to_completed" and not fields.get("completion_path"):
            fields["completion_path"] = "report" if str(fields.get("report", "")).strip() else "no_update"

        transition = next((t for t in TRANSITIONS if t["id"] == transition_id), None)
        if not transition:
            raise WorkflowError("Unknown workflow transition")
        if transition["source_type"] != entity_type:
            raise WorkflowError("This transition does not apply to this item")

        # Validate required fields.
        for field_def in transition["fields"]:
            name = str(field_def["name"])
            if field_def.get("required") and not str(fields.get(name, "")).strip():
                raise WorkflowError(f"{field_def['label']} is required")
            condition = field_def.get("required_when")
            if condition and all(str(fields.get(key, "")) == str(value) for key, value in condition.items()) and not str(fields.get(name, "")).strip():
                raise WorkflowError(f"{field_def['label']} is required")

        if transition_id == "capture_to_problem":
            return self.promote_capture(entity_id, str(fields.get("statement", "")), str(fields.get("detail", "")))

        if transition_id == "problem_to_solution":
            problem = self.db.execute("SELECT state FROM problems WHERE id=?", (entity_id,)).fetchone()
            if not problem:
                raise WorkflowError("Problem not found")
            if problem[0] != "approved":
                raise WorkflowError("Features require an approved problem")
            return self.create_feature(
                entity_id,
                str(fields.get("title", "")),
                str(fields.get("outcome", "")),
                str(fields.get("non_goals", "")),
                str(fields.get("validation_criteria", "")),
            )

        if transition_id == "solution_to_approved":
            return self._transition_approve_solution(entity_id, fields)

        if transition_id == "solution_to_completed":
            return self._transition_complete_solution(entity_id, fields)

        raise WorkflowError("Transition not implemented")

    def _transition_approve_solution(self, feature_id: str, fields: dict[str, object]) -> dict[str, object]:
        """Approve a Solution, optionally skipping the conflict check with a reason."""
        feature = self.db.execute("SELECT state, conflict_state FROM features WHERE id=?", (feature_id,)).fetchone()
        if not feature:
            raise WorkflowError("Solution not found")
        if feature["state"] == "approved":
            raise WorkflowError("This Solution is already approved")

        approval_path = str(fields.get("approval_path", "checked"))
        if approval_path not in {"checked", "skip"}:
            raise WorkflowError("Invalid conflict-check path")
        skip_conflict = approval_path == "skip" or bool(fields.get("skip_conflict_check"))
        conflict_state = str(fields.get("_legacy_conflict_state", "clear"))
        citation = str(fields.get("citation", ""))
        skip_reason = str(fields.get("skip_reason", ""))

        if skip_conflict:
            if not skip_reason.strip():
                raise WorkflowError("Skipping the conflict check requires a reason")
            # Record a human-reviewed conflict decision that documents the skip.
            self.record_conflict_evaluation(feature_id, "clear", f"Conflict check skipped: {skip_reason.strip()}")
        else:
            if conflict_state not in {"clear", "conflicted", "unknown"}:
                raise WorkflowError("Invalid conflict state")
            if conflict_state == "clear" and not citation.strip():
                raise WorkflowError("A clear evaluation requires a cited current-context basis")
            if conflict_state != "clear":
                # Record the conflict evaluation but do not approve; the human must resolve it.
                self.record_conflict_evaluation(feature_id, conflict_state, citation)
                return {"feature_id": feature_id, "state": conflict_state, "approved": False}
            self.record_conflict_evaluation(feature_id, "clear", citation)

        self.approve_feature(feature_id)
        return {"feature_id": feature_id, "state": "clear", "approved": True}

    def _transition_complete_solution(self, feature_id: str, fields: dict[str, object]) -> dict[str, object]:
        """Record completion and complete the Problem, allowing a no-report path with a reason."""
        feature = self.db.execute("SELECT state, problem_id FROM features WHERE id=?", (feature_id,)).fetchone()
        if not feature:
            raise WorkflowError("Solution not found")
        if feature["state"] != "approved":
            raise WorkflowError("Completion requires an in-progress (approved) Solution")

        evidence = str(fields.get("evidence", "")).strip()
        report = str(fields.get("report", "")).strip()
        no_update_reason = str(fields.get("no_update_reason", "")).strip()
        reason = str(fields.get("reason", "")).strip()

        if not evidence:
            raise WorkflowError("Implementation evidence is required")
        # A report is required unless a no-update reason is given.
        if not report and not no_update_reason:
            raise WorkflowError("A completion report or a no-update reason is required")

        self.record_completion(feature_id, evidence, report or "No report recorded.", no_update_reason)
        # The transition is an explicit human completion decision. When a report
        # is provided, it serves as the knowledge record, so mark it as integrated
        # so verify_completion's knowledge-status gate is satisfied. When a
        # no_update_reason is given, record_completion already sets "not_needed".
        if report and not no_update_reason:
            self.db.execute("UPDATE completions SET knowledge_status='integrated' WHERE feature_id=?", (feature_id,))
            self.db.commit()
        # Verify the completion so the Problem can be completed.
        self.verify_completion(feature_id)
        # Complete the Problem with the optional reason.
        self.complete_problem(feature["problem_id"], reason)
        return {
            "feature_id": feature_id,
            "problem_id": feature["problem_id"],
            "completed": True,
            "note_skipped": bool(no_update_reason) and not bool(report),
        }



    def board(self, locale: str = "en") -> dict[str, list[dict[str, object]]]:
        return {
            "captures": self._active("captures"),
            "problems": self.localized.overlay_many("problems", self._active("problems"), locale),
            "features": self.localized.overlay_many("features", self._active("features"), locale),
        }

    def problem_record(self, problem_id: str) -> dict[str, str]:
        row = self.db.execute("SELECT * FROM problems WHERE id=?", (problem_id,)).fetchone()
        if not row:
            raise WorkflowError("Problem not found")
        return dict(row)

    def _active(self, entity_type: str) -> list[dict[str, str]]:
        # The workbench is a view of current work. Completed history remains in SQLite
        # (and is still available to reports), but must not compete with active cards.
        state_filter = "" if entity_type == "captures" else " AND item.state NOT IN ('archived', 'completed')"
        lineage_filter = ""
        if entity_type == "features":
            lineage_filter = " AND NOT EXISTS (SELECT 1 FROM problems parent WHERE parent.id=item.problem_id AND parent.state='completed')"
        rows = self.db.execute(
            f"""SELECT item.*, COALESCE(priority.category, '') AS category,
                       COALESCE(priority.attention_rank, 0) AS attention_rank,
                       COALESCE(priority.rationale, '') AS attention_rationale,
                       COALESCE(override.manual_priority, 0) AS manual_priority
                FROM {entity_type} item
                LEFT JOIN workbench_priorities priority
                  ON priority.entity_type=? AND priority.entity_id=item.id
                LEFT JOIN workbench_priority_overrides override
                  ON override.entity_type=? AND override.entity_id=item.id
                WHERE NOT EXISTS (SELECT 1 FROM deleted_entities deleted WHERE deleted.entity_type=? AND deleted.entity_id=item.id){state_filter}{lineage_filter}
                ORDER BY attention_rank DESC, item.created_at DESC""",
            (entity_type, entity_type, entity_type),
        ).fetchall()
        return [dict(row) for row in rows]

    @staticmethod
    def _category(text: str) -> str:
        normalized = text.lower()
        if "llm wiki" in normalized or "llm-wiki" in normalized:
            return "LLM Wiki"
        return "General"

    def organize_workbench(self) -> None:
        """Persist a transparent local attention ordering; it never changes workflow state."""
        problems = self._active("problems")
        problem_categories = {problem["id"]: self._category(problem["statement"]) for problem in problems}
        importance = {row["problem_id"]: int(row["importance"]) for row in self.db.execute("SELECT problem_id, importance FROM importance_assessments")}
        candidates: list[tuple[str, str, str, int, str]] = []
        for capture in self._active("captures"):
            override = self.db.execute("SELECT category FROM workbench_category_overrides WHERE entity_type='captures' AND entity_id=?", (capture["id"],)).fetchone()
            category = str(override[0]) if override else self._category(capture["text"])
            candidates.append(("captures", capture["id"], category, 60, "Untriaged inbox item"))
        for problem in problems:
            base = 100 if problem["state"] == "draft" else 75
            score = base + importance.get(problem["id"], 0)
            reason = "Needs a human decision" if problem["state"] == "draft" else "Approved direction"
            override = self.db.execute("SELECT category FROM workbench_category_overrides WHERE entity_type='problems' AND entity_id=?", (problem["id"],)).fetchone()
            category = str(override[0]) if override else problem_categories[problem["id"]]
            candidates.append(("problems", problem["id"], category, score, reason))
        for feature in self._active("features"):
            override = self.db.execute("SELECT category FROM workbench_category_overrides WHERE entity_type='features' AND entity_id=?", (feature["id"],)).fetchone()
            category = str(override[0]) if override else problem_categories.get(feature["problem_id"], self._category(feature["title"]))
            base = 95 if feature["state"] == "approved" else 70
            score = base + (10 if feature["conflict_state"] == "clear" else 0)
            reason = "Approved work in progress" if feature["state"] == "approved" else "Needs conflict review or approval"
            candidates.append(("features", feature["id"], category, score, reason))
        self.db.executemany(
            """INSERT INTO workbench_priorities(entity_type,entity_id,category,attention_rank,rationale)
               VALUES (?,?,?,?,?)
               ON CONFLICT(entity_type,entity_id) DO UPDATE SET category=excluded.category,
                 attention_rank=excluded.attention_rank,rationale=excluded.rationale,updated_at=CURRENT_TIMESTAMP""",
            candidates,
        )
        self.db.commit()

    def apply_ai_organization(self, entries: object) -> int:
        """Apply a reviewed structured organization response without changing workflow state."""
        if not isinstance(entries, list):
            raise WorkflowError("AI organization response must contain an entries list")
        valid_ids = {
            (entity_type, row["id"])
            for entity_type in ("captures", "problems", "features")
            for row in self._active(entity_type)
        }
        updates: list[tuple[str, int, str, str, str]] = []
        for entry in entries:
            if not isinstance(entry, dict):
                continue
            entity_type, entity_id = str(entry.get("entity_type", "")), str(entry.get("entity_id", ""))
            if (entity_type, entity_id) not in valid_ids:
                continue
            category = str(entry.get("category", "General")).strip()[:80] or "General"
            category_override = self.db.execute("SELECT category FROM workbench_category_overrides WHERE entity_type=? AND entity_id=?", (entity_type, entity_id)).fetchone()
            if category_override:
                category = str(category_override[0])
            try:
                rank = max(0, min(100, int(entry.get("attention_rank", 0))))
            except (TypeError, ValueError):
                rank = 0
            rationale = str(entry.get("rationale", "AI-organized attention priority")).strip()[:400]
            override = self.db.execute("SELECT manual_priority FROM workbench_priority_overrides WHERE entity_type=? AND entity_id=?", (entity_type, entity_id)).fetchone()
            if override and int(override[0]) == 1:
                rank = max(rank, 90)
                rationale = "Manually marked important"
            elif override and int(override[0]) == -1:
                rank = min(rank, 89)
                rationale = "Manually marked not important"
            updates.append((entity_type, entity_id, category, rank, rationale))
        if not updates:
            raise WorkflowError("AI organization did not return usable workbench items")
        self.db.executemany(
            """INSERT INTO workbench_priorities(entity_type,entity_id,category,attention_rank,rationale)
               VALUES (?,?,?,?,?)
               ON CONFLICT(entity_type,entity_id) DO UPDATE SET category=excluded.category,
                 attention_rank=excluded.attention_rank,rationale=excluded.rationale,updated_at=CURRENT_TIMESTAMP""",
            updates,
        )
        self.db.commit()
        return len(updates)

    def set_workbench_category(self, entity_type: str, entity_id: str, category: str) -> None:
        if entity_type not in {"captures", "problems", "features"}:
            raise WorkflowError("Unsupported workbench item")
        if not self.db.execute(f"SELECT 1 FROM {entity_type} WHERE id=?", (entity_id,)).fetchone():
            raise WorkflowError("Workbench item not found")
        label = category.strip()[:80]
        if not label:
            raise WorkflowError("A category is required")
        for linked_type, linked_id in self._linked_workbench_items(entity_type, entity_id):
            existing = self.db.execute(
                "SELECT attention_rank,rationale FROM workbench_priorities WHERE entity_type=? AND entity_id=?",
                (linked_type, linked_id),
            ).fetchone()
            rank, rationale = (int(existing[0]), str(existing[1])) if existing else (0, "Manually categorized")
            self.db.execute(
                """INSERT INTO workbench_priorities(entity_type,entity_id,category,attention_rank,rationale)
                   VALUES (?,?,?,?,?)
                   ON CONFLICT(entity_type,entity_id) DO UPDATE SET category=excluded.category,updated_at=CURRENT_TIMESTAMP""",
                (linked_type, linked_id, label, rank, rationale),
            )
            # A drag is an explicit category decision for the complete lineage. It
            # intentionally replaces earlier overrides, including explicit General.
            self.db.execute(
                "INSERT OR REPLACE INTO workbench_category_overrides(entity_type,entity_id,category) VALUES (?,?,?)",
                (linked_type, linked_id, label),
            )
        self.db.commit()

    def _inherit_workbench_category(
        self,
        source_type: str,
        source_id: str,
        target_type: str,
        target_id: str,
        fallback_text: str,
    ) -> None:
        """Give a newly linked item its parent's effective category.

        Manual overrides are copied as overrides so an explicitly selected General
        remains General through later workflow transitions. Automatically assigned
        categories are copied only to priorities and remain eligible for organizing.
        """
        override = self.db.execute(
            "SELECT category FROM workbench_category_overrides WHERE entity_type=? AND entity_id=?",
            (source_type, source_id),
        ).fetchone()
        priority = self.db.execute(
            "SELECT category FROM workbench_priorities WHERE entity_type=? AND entity_id=?",
            (source_type, source_id),
        ).fetchone()
        category = str(override[0]) if override else str(priority[0]) if priority else self._category(fallback_text)
        self.db.execute(
            """INSERT INTO workbench_priorities(entity_type,entity_id,category,attention_rank,rationale)
               VALUES (?,?,?,?,?)
               ON CONFLICT(entity_type,entity_id) DO UPDATE SET category=excluded.category,updated_at=CURRENT_TIMESTAMP""",
            (target_type, target_id, category, 0, f"Inherited from linked {source_type[:-1].title()}"),
        )
        if override:
            self.db.execute(
                "INSERT OR REPLACE INTO workbench_category_overrides(entity_type,entity_id,category) VALUES (?,?,?)",
                (target_type, target_id, category),
            )

    def _linked_workbench_items(self, entity_type: str, entity_id: str) -> list[tuple[str, str]]:
        """Return the complete Capture -> Problem -> Solution lineage for an item."""
        capture_id: str | None = None
        problem_id: str | None = None
        if entity_type == "captures":
            capture_id = entity_id
            problem = self.db.execute("SELECT id FROM problems WHERE capture_id=?", (capture_id,)).fetchone()
            problem_id = str(problem[0]) if problem else None
        elif entity_type == "problems":
            problem = self.db.execute("SELECT capture_id FROM problems WHERE id=?", (entity_id,)).fetchone()
            problem_id = entity_id
            capture_id = str(problem[0]) if problem else None
        else:
            feature = self.db.execute("SELECT problem_id FROM features WHERE id=?", (entity_id,)).fetchone()
            problem_id = str(feature[0]) if feature else None
            if problem_id:
                problem = self.db.execute("SELECT capture_id FROM problems WHERE id=?", (problem_id,)).fetchone()
                capture_id = str(problem[0]) if problem else None

        linked: list[tuple[str, str]] = []
        if capture_id:
            linked.append(("captures", capture_id))
        if problem_id:
            linked.append(("problems", problem_id))
            linked.extend(
                ("features", str(row[0]))
                for row in self.db.execute("SELECT id FROM features WHERE problem_id=?", (problem_id,))
            )
        return linked

    def set_workbench_importance(self, entity_type: str, entity_id: str, important: bool) -> None:
        if entity_type not in {"captures", "problems", "features"}:
            raise WorkflowError("Unsupported workbench item")
        if not self.db.execute(f"SELECT 1 FROM {entity_type} WHERE id=?", (entity_id,)).fetchone():
            raise WorkflowError("Workbench item not found")
        self.db.execute(
            "INSERT OR REPLACE INTO workbench_priority_overrides(entity_type,entity_id,manual_priority) VALUES (?,?,?)",
            (entity_type, entity_id, 1 if important else -1),
        )
        self.db.commit()

    def solution_progress(self, feature_id: str, locale: str = "en") -> dict[str, list[dict[str, object]]]:
        feature = self.db.execute("SELECT validation_criteria FROM features WHERE id=?", (feature_id,)).fetchone()
        if not feature:
            raise WorkflowError("Solution not found")
        self.seed_solution_checklist(feature_id, str(feature[0] or ""))
        entry_rows = list(self.db.execute(
            "SELECT * FROM solution_progress_entries WHERE feature_id=? ORDER BY created_at DESC", (feature_id,)
        ))
        entries = self.localized.overlay_many("solution_progress_entries", entry_rows, locale)
        for entry in entries:
            entry["comments"] = [dict(row) for row in self.db.execute(
                "SELECT * FROM solution_progress_comments WHERE entry_id=? ORDER BY created_at", (entry["id"],)
            )]
        checklist = [dict(row) for row in self.db.execute(
            "SELECT * FROM solution_checklist_items WHERE feature_id=? ORDER BY checked, created_at", (feature_id,)
        )]
        return {"entries": entries, "checklist": checklist}

    def add_solution_progress(self, feature_id: str, body: str = "", image_data: str = "", image_media_type: str = "") -> dict[str, object]:
        feature = self.db.execute("SELECT state FROM features WHERE id=?", (feature_id,)).fetchone()
        if not feature:
            raise WorkflowError("Solution not found")
        if feature[0] != "approved":
            raise WorkflowError("Progress records can be added only to an In Progress Solution")
        if not body.strip() and not image_data:
            raise WorkflowError("Add a short note or paste an image")
        entry_id = str(uuid.uuid4())
        self.db.execute(
            "INSERT INTO solution_progress_entries(id,feature_id,body,image_data,image_media_type) VALUES (?,?,?,?,?)",
            (entry_id, feature_id, body.strip(), image_data, image_media_type),
        )
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM solution_progress_entries WHERE id=?", (entry_id,)).fetchone())

    def add_solution_comment(self, entry_id: str, body: str) -> dict[str, object]:
        if not body.strip():
            raise WorkflowError("Comment cannot be empty")
        if not self.db.execute("SELECT 1 FROM solution_progress_entries WHERE id=?", (entry_id,)).fetchone():
            raise WorkflowError("Progress record not found")
        comment_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO solution_progress_comments(id,entry_id,body) VALUES (?,?,?)", (comment_id, entry_id, body.strip()))
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM solution_progress_comments WHERE id=?", (comment_id,)).fetchone())

    def add_solution_checklist_item(self, feature_id: str, body: str) -> dict[str, object]:
        if not body.strip():
            raise WorkflowError("Checklist item cannot be empty")
        if not self.db.execute("SELECT 1 FROM features WHERE id=?", (feature_id,)).fetchone():
            raise WorkflowError("Solution not found")
        item_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO solution_checklist_items(id,feature_id,body) VALUES (?,?,?)", (item_id, feature_id, body.strip()))
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM solution_checklist_items WHERE id=?", (item_id,)).fetchone())

    def seed_solution_checklist(self, feature_id: str, validation_criteria: str) -> int:
        """Import only Validation Criteria bullets once, never general Solution bullets."""
        existing = self.db.execute("SELECT count(*) FROM solution_checklist_items WHERE feature_id=?", (feature_id,)).fetchone()
        if not existing or existing[0]:
            return 0
        found: list[tuple[str, bool]] = []
        seen: set[str] = set()
        pattern = re.compile(r"^\s*(?:[-*+]\s+|\d+[.)]\s+)(?:\[\s*([xX ])\s*\]\s*)?(.+?)\s*$")
        for line in validation_criteria.splitlines():
            match = pattern.match(line)
            if not match:
                continue
            body = match.group(2).strip()
            normalized = body.casefold()
            if not body or normalized in seen:
                continue
            seen.add(normalized)
            found.append((body, (match.group(1) or "").lower() == "x"))
        self.db.executemany(
            "INSERT INTO solution_checklist_items(id,feature_id,body,checked) VALUES (?,?,?,?)",
            [(str(uuid.uuid4()), feature_id, body, int(checked)) for body, checked in found[:30]],
        )
        self.db.commit()
        return len(found[:30])

    def update_solution_checklist_item(self, item_id: str, body: str, checked: bool) -> None:
        if not body.strip():
            raise WorkflowError("Checklist item cannot be empty")
        if self.db.execute("UPDATE solution_checklist_items SET body=?,checked=?,updated_at=CURRENT_TIMESTAMP WHERE id=?", (body.strip(), int(checked), item_id)).rowcount != 1:
            raise WorkflowError("Checklist item not found")
        self.db.commit()

    def set_solution_progress_summary(self, entry_id: str, summary: str) -> None:
        self.db.execute("UPDATE solution_progress_entries SET image_summary=? WHERE id=?", (summary.strip(), entry_id))
        self.db.commit()

    def set_solution_progress_summaries(
        self,
        entry_id: str,
        versions: dict[str, dict[str, str]],
        locale: str,
    ) -> None:
        """Atomically persist both generated summaries and one compatibility value."""
        if not self.db.execute("SELECT 1 FROM solution_progress_entries WHERE id=?", (entry_id,)).fetchone():
            raise WorkflowError("Progress record not found")
        if locale not in {"ko", "en"}:
            raise ValueError("Unsupported locale; expected 'ko' or 'en'")
        if set(versions) != {"ko", "en"}:
            raise ValueError("Image Summary requires complete Korean and English versions")
        summaries = {
            item_locale: str(fields.get("image_summary", "")).strip()
            for item_locale, fields in versions.items()
        }
        if any(not summary for summary in summaries.values()):
            raise ValueError("Image Summary versions cannot be empty")
        normalized = {
            item_locale: {"image_summary": summaries[item_locale]}
            for item_locale in ("ko", "en")
        }
        try:
            self.localized.save_bilingual("solution_progress_entries", entry_id, normalized)
            self.db.execute(
                "UPDATE solution_progress_entries SET image_summary=? WHERE id=?",
                (summaries[locale], entry_id),
            )
            self.db.commit()
        except Exception:
            self.db.rollback()
            raise

    def recent_completed_solutions(self, limit: int = 5, locale: str = "en") -> list[dict[str, object]]:
        """Human-verified work with the generated document that preserves it."""
        rows = self.db.execute(
            """SELECT f.*, COALESCE(c.evidence, '') AS completion_evidence, COALESCE(c.report, '') AS completion_report,
                      COALESCE(c.created_at, '') AS completed_at, COALESCE(m.path, '') AS generated_path,
                      p.id AS problem_id, p.statement AS problem_statement,
                      COALESCE(cp.path, '') AS completion_playbook_path
               FROM features f
               JOIN problems p ON p.id=f.problem_id
               LEFT JOIN completions c ON c.feature_id=f.id
               LEFT JOIN mirror_files m ON m.entity_type='features' AND m.entity_id=f.id
               LEFT JOIN completion_playbooks cp ON cp.problem_id=p.id
               WHERE c.state='verified' OR cp.path IS NOT NULL OR p.state='completed'
               ORDER BY COALESCE(c.created_at, cp.created_at) DESC LIMIT ?""",
            (limit,),
        ).fetchall()
        solutions = self.localized.overlay_many("features", rows, locale)
        problems = {
            str(row["id"]): row
            for row in self.localized.overlay_many(
                "problems",
                self.db.execute(
                    f"SELECT * FROM problems WHERE id IN ({','.join('?' for _ in solutions)})",
                    tuple(str(row["problem_id"]) for row in solutions),
                ).fetchall(),
                locale,
            )
        } if solutions else {}
        for solution in solutions:
            problem = problems.get(str(solution["problem_id"]))
            if problem:
                solution["problem_statement"] = problem["statement"]
        return solutions

    def delete(self, entity_type: str, entity_id: str) -> None:
        if entity_type not in {"captures", "problems", "features"}:
            raise WorkflowError("Unsupported deletion type")
        if not self.db.execute(f"SELECT 1 FROM {entity_type} WHERE id=?", (entity_id,)).fetchone():
            raise WorkflowError("Item not found")
        items = [(entity_type, entity_id)]
        if entity_type == "problems":
            feature_ids = [row[0] for row in self.db.execute("SELECT id FROM features WHERE problem_id=?", (entity_id,))]
            items.extend(("features", feature_id) for feature_id in feature_ids)
        self.db.executemany("INSERT OR REPLACE INTO deleted_entities(entity_type,entity_id) VALUES (?,?)", items)
        self.db.commit()

    def restore(self, entity_type: str, entity_id: str) -> None:
        self.db.execute("DELETE FROM deleted_entities WHERE entity_type=? AND entity_id=?", (entity_type, entity_id))
        self.db.commit()

    def context_for(self, entity_type: str, entity_id: str, locale: str = "en") -> dict[str, str]:
        if entity_type not in {"captures", "problems", "features"}:
            raise WorkflowError("Unsupported workflow item")
        row = self.db.execute(f"SELECT * FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
        if not row:
            raise WorkflowError("Item not found")
        values = self.localized.overlay(entity_type, row, locale) if entity_type in {"problems", "features"} else dict(row)
        title = values.get("statement") or values.get("title") or values.get("outcome") or values.get("text") or "Workflow item"
        detail = values.get("detail") or values.get("outcome") or values.get("text") or ""
        return {"type": entity_type[:-1], "title": str(title), "detail": str(detail)}

    def update_manual(self, entity_type: str, entity_id: str, title: str, detail: str, localized_versions: dict[str, dict[str, str]] | None = None) -> None:
        if entity_type == "captures":
            sql, values = "UPDATE captures SET text=? WHERE id=?", (title, entity_id)
        elif entity_type == "problems":
            sql, values = "UPDATE problems SET statement=?,detail=? WHERE id=?", (title, detail, entity_id)
        elif entity_type == "features":
            current = self.db.execute("SELECT title,outcome FROM features WHERE id=?", (entity_id,)).fetchone()
            if not current:
                raise WorkflowError("Item not found")
            sql, values = "UPDATE features SET title=?,outcome=? WHERE id=?", (title, detail, entity_id)
        else:
            raise WorkflowError("Unsupported manual update type")
        if self.db.execute(sql, values).rowcount != 1:
            raise WorkflowError("Item not found")
        if localized_versions and entity_type in {"problems", "features"}:
            mapped = {
                locale: (
                    {"statement": fields.get("statement", fields.get("title", "")), "detail": fields.get("detail", "")}
                    if entity_type == "problems"
                    else {"title": fields.get("title", ""), "outcome": fields.get("outcome", fields.get("detail", ""))}
                )
                for locale, fields in localized_versions.items()
            }
            self.localized.save_versions(entity_type, entity_id, mapped, complete=False)
        if entity_type == "features" and (current["title"] != title or current["outcome"] != detail):
            self._record_solution_decision(
                entity_id,
                "manual_edit",
                {"title": current["title"], "outcome": current["outcome"]},
                {"title": title, "outcome": detail},
                "",
                "human_action",
                entity_id,
            )
        self.db.commit()

    def record_ai_run(self, entity_type: str, entity_id: str, kind: str, input_text: str, output_text: str) -> None:
        self.db.execute("INSERT INTO ai_runs(id,entity_type,entity_id,kind,input_text,output_text) VALUES (?,?,?,?,?,?)", (str(uuid.uuid4()), entity_type, entity_id, kind, input_text, output_text))
        self.db.commit()

    def _context_lineage_sources(self, entity_type: str, entity_id: str) -> list[tuple[str, str, str]]:
        sources = [(entity_type, entity_id, "Current")]
        problem_id = entity_id if entity_type == "problems" else ""
        if entity_type == "features":
            linked_problem = self.db.execute("SELECT problem_id FROM features WHERE id=?", (entity_id,)).fetchone()
            if linked_problem and linked_problem[0]:
                problem_id = str(linked_problem[0])
                sources.append(("problems", problem_id, "Problem"))
        if problem_id:
            linked_capture = self.db.execute("SELECT capture_id FROM problems WHERE id=?", (problem_id,)).fetchone()
            if linked_capture and linked_capture[0]:
                sources.append(("captures", str(linked_capture[0]), "Capture"))
            completed_source = self.db.execute(
                "SELECT source_feature_id FROM follow_up_links WHERE problem_id=?", (problem_id,)
            ).fetchone()
            if completed_source and completed_source[0]:
                sources.append(("features", str(completed_source[0]), "Completed Solution"))
        return sources

    def chat_history(self, entity_type: str, entity_id: str, limit: int = 6) -> list[dict[str, str]]:
        sources = self._context_lineage_sources(entity_type, entity_id)
        source_clause = " OR ".join("(entity_type=? AND entity_id=?)" for _ in sources)
        source_values = tuple(value for source in sources for value in source[:2])
        rows = self.db.execute(
            f"""SELECT input_text,output_text FROM ai_runs
                WHERE kind='workflow_chat' AND ({source_clause})
                ORDER BY created_at DESC,rowid DESC LIMIT ?""",
            (*source_values, limit),
        ).fetchall()
        history: list[dict[str, str]] = []
        for row in reversed(rows):
            history.extend([{"role": "user", "content": row[0]}, {"role": "assistant", "content": row[1]}])
        return history

    @staticmethod
    def _refinement_context_text(value: str) -> str:
        text = str(value or "").strip()
        if text.startswith("{"):
            parsed: object = text
            try:
                parsed = json.loads(text)
            except (TypeError, ValueError):
                try:
                    parsed = ast.literal_eval(text)
                except (SyntaxError, ValueError):
                    pass
            if isinstance(parsed, dict):
                text = ". ".join(str(item) for item in parsed.values() if str(item).strip())
        text = re.sub(r"^\s*(?:#{1,6}|[-*>])\s*", "", text, flags=re.MULTILINE)
        text = text.replace("**", "").replace("`", "")
        return re.sub(r"\s+", " ", text).strip()

    def refinement_context_summary(
        self,
        entity_type: str,
        entity_id: str,
        max_entries: int = 5,
        max_characters: int = 500,
        locale: str = "en",
    ) -> dict[str, object]:
        """Return bounded, deterministic context for a workflow Preview."""
        if entity_type not in {"captures", "problems", "features"}:
            raise WorkflowError("Refinement Preview context supports Capture, Problem, and Solution items only")
        context = self.context_for(entity_type, entity_id, locale)
        title = self._refinement_context_text(context["title"])
        detail = self._refinement_context_text(context["detail"])
        boilerplate = {"n/a", "none", "not yet known", "unknown", "tbd", "to be determined"}
        candidates: list[tuple[str, str]] = []
        if title:
            candidates.append(("Current item", title))
        if detail and detail.casefold() != title.casefold() and detail.casefold().rstrip(".!") not in boilerplate:
            candidates.append(("Current context", detail))

        sources = self._context_lineage_sources(entity_type, entity_id)
        source_labels = {(source_type, source_id): label for source_type, source_id, label in sources}
        source_clause = " OR ".join("(entity_type=? AND entity_id=?)" for _ in sources)
        source_values = tuple(value for source in sources for value in source[:2])
        rows = self.db.execute(
            f"""SELECT entity_type,entity_id,kind,input_text,output_text FROM ai_runs
                WHERE ({source_clause}) AND kind IN ('workflow_chat','workflow_refinement')
                ORDER BY created_at DESC,rowid DESC LIMIT ?""",
            (*source_values, max_entries * 2),
        ).fetchall()
        first_row_by_source: dict[tuple[str, str], int] = {}
        for index, row in enumerate(rows):
            first_row_by_source.setdefault((str(row[0]), str(row[1])), index)
        priority_indices = [
            first_row_by_source[(source_type, source_id)]
            for source_type, source_id, _ in sources
            if (source_type, source_id) in first_row_by_source
        ]
        remaining_indices = [index for index in range(len(rows)) if index not in priority_indices]

        def append_run(row: sqlite3.Row) -> None:
            lineage_label = source_labels.get((str(row[0]), str(row[1])), "Current")
            if row[2] == "workflow_refinement":
                label = "Previous preview" if lineage_label == "Current" else f"Earlier {lineage_label} refinement"
                candidates.append((label, self._refinement_context_text(row[4])))
            else:
                question = self._refinement_context_text(row[3])
                answer = self._refinement_context_text(row[4])
                label = "Recent discussion" if lineage_label == "Current" else f"Earlier {lineage_label} discussion"
                candidates.append((label, " — ".join(part for part in (question, answer) if part)))

        for index in priority_indices:
            append_run(rows[index])
        for source_type, source_id, lineage_label in sources[1:]:
            if (source_type, source_id) in first_row_by_source:
                continue
            table, field = {
                "problems": ("problems", "statement"),
                "captures": ("captures", "text"),
                "features": ("features", "title"),
            }[source_type]
            source_row = self.db.execute(f"SELECT {field} FROM {table} WHERE id=?", (source_id,)).fetchone()
            source_text = self._refinement_context_text(source_row[0]) if source_row else ""
            if source_text and source_text.casefold() != title.casefold():
                candidates.append((f"Source {lineage_label}", source_text))
        for index in remaining_indices:
            append_run(rows[index])

        entries: list[dict[str, str]] = []
        seen: set[str] = set()
        remaining = max(0, max_characters)
        per_entry_limit = min(160, max_characters)
        for label, text in candidates:
            normalized = self._refinement_context_text(text)
            key = normalized.casefold()
            if not normalized or key in seen or remaining <= 0 or len(entries) >= max_entries:
                continue
            seen.add(key)
            allowed = min(remaining, per_entry_limit)
            if len(normalized) > allowed:
                normalized = "…" if allowed == 1 else normalized[: allowed - 1].rstrip() + "…"
            entries.append({"label": label, "text": normalized})
            remaining -= len(normalized)

        assessment = self.refinement_structure_assessment(entity_type, entity_id) if entity_type != "captures" else {}
        refinement_draft = self.latest_refinement_draft(entity_type, entity_id) if entity_type != "captures" else None
        next_draft = self.latest_next_draft(entity_type, entity_id)
        return {
            "has_context": bool(entries),
            "entries": entries,
            "current_detail": self.current_item_detail(entity_type, entity_id, locale),
            "refinement_draft": refinement_draft,
            "next_draft": next_draft,
            **assessment,
        }

    def current_item_detail(self, entity_type: str, entity_id: str, locale: str = "en") -> dict[str, object]:
        """Return the saved Item Detail independently of generated refinement history."""
        if entity_type == "captures":
            row = self.db.execute("SELECT text,created_at FROM captures WHERE id=?", (entity_id,)).fetchone()
            if not row:
                raise WorkflowError("Item not found")
            return {
                "kind": "capture",
                "title": str(row["text"]),
                "detail": str(row["text"]),
                "state": "captured",
                "created_at": str(row["created_at"]),
            }
        if entity_type == "problems":
            row = self.db.execute(
                "SELECT statement,detail,state,created_at FROM problems WHERE id=?",
                (entity_id,),
            ).fetchone()
            if not row:
                raise WorkflowError("Item not found")
            values = self.localized.overlay("problems", {"id": entity_id, **dict(row)}, locale)
            return {
                "kind": "problem",
                "title": str(values["statement"]),
                "detail": str(values["detail"] or ""),
                "state": str(row["state"]),
                "created_at": str(row["created_at"]),
            }
        if entity_type == "features":
            row = self.db.execute(
                """SELECT f.title,f.outcome,f.non_goals,f.validation_criteria,f.state,
                          f.conflict_state,f.created_at,f.problem_id,p.statement AS problem_statement
                   FROM features f JOIN problems p ON p.id=f.problem_id WHERE f.id=?""",
                (entity_id,),
            ).fetchone()
            if not row:
                raise WorkflowError("Item not found")
            values = self.localized.overlay("features", {"id": entity_id, **dict(row)}, locale)
            problem = self.db.execute("SELECT * FROM problems WHERE id=?", (row["problem_id"],)).fetchone()
            if problem:
                values["problem_statement"] = self.localized.overlay("problems", problem, locale)["statement"]
            return {
                "kind": "solution",
                "title": values["title"],
                "outcome": values["outcome"],
                "non_goals": values["non_goals"],
                "validation_criteria": values["validation_criteria"],
                "state": values["state"],
                "conflict_state": values["conflict_state"],
                "created_at": values["created_at"],
                "problem_id": values["problem_id"],
                "problem_statement": values["problem_statement"],
            }
        raise WorkflowError("Item Detail supports Capture, Problem, and Solution items only")

    def latest_refinement_draft(self, entity_type: str, entity_id: str) -> dict[str, object] | None:
        """Restore the latest generated refinement so Preview survives closing the chat."""
        if entity_type not in {"problems", "features"}:
            raise WorkflowError("Refinement Preview draft supports Problem and Solution items only")
        row = self.db.execute(
            """SELECT output_text FROM ai_runs
               WHERE entity_type=? AND entity_id=? AND kind='workflow_refinement'
               ORDER BY created_at DESC,rowid DESC LIMIT 1""",
            (entity_type, entity_id),
        ).fetchone()
        if not row:
            return None
        value: object = row[0]
        try:
            value = json.loads(str(value))
        except (TypeError, ValueError):
            try:
                value = ast.literal_eval(str(value))
            except (SyntaxError, ValueError):
                return None
        if not isinstance(value, dict):
            return None
        title = str(value.get("title") or "").strip()
        detail = str(value.get("detail") or "").strip()
        if not title or not detail:
            return None
        current = self.context_for(entity_type, entity_id)
        restored = {
            "title": title,
            "detail": detail,
            "applied": title == current["title"].strip() and detail == current["detail"].strip(),
        }
        if value.get("localized_versions"):
            restored["localized_versions"] = value["localized_versions"]
        return restored

    def latest_next_draft(self, entity_type: str, entity_id: str) -> dict[str, object] | None:
        """Restore the latest proposed next-stage item generated from a Capture or Problem."""
        if entity_type not in {"captures", "problems"}:
            return None
        row = self.db.execute(
            """SELECT output_text FROM ai_runs
               WHERE entity_type=? AND entity_id=? AND kind='workflow_draft'
               ORDER BY created_at DESC,rowid DESC LIMIT 1""",
            (entity_type, entity_id),
        ).fetchone()
        if not row:
            return None
        value: object = row[0]
        try:
            value = json.loads(str(value))
        except (TypeError, ValueError):
            try:
                value = ast.literal_eval(str(value))
            except (SyntaxError, ValueError):
                return None
        if not isinstance(value, dict):
            return None
        fields = ("title", "detail") if entity_type == "captures" else ("title", "outcome", "non_goals", "validation_criteria")
        draft = {field: str(value.get(field) or "").strip() for field in fields}
        if any(not draft[field] for field in fields):
            return None
        if entity_type == "captures":
            applied = self.db.execute(
                "SELECT 1 FROM problems WHERE capture_id=? AND statement=? AND detail=? LIMIT 1",
                (entity_id, draft["title"], draft["detail"]),
            ).fetchone()
        else:
            applied = self.db.execute(
                """SELECT 1 FROM features
                   WHERE problem_id=? AND title=? AND outcome=? AND non_goals=? AND validation_criteria=?
                   LIMIT 1""",
                (entity_id, *(draft[field] for field in fields)),
            ).fetchone()
        restored = {**draft, "applied": bool(applied)}
        if value.get("localized_versions"):
            restored["localized_versions"] = value["localized_versions"]
        return restored

    def completed_solution(self, feature_id: str, locale: str = "en") -> dict[str, object]:
        """Return one immutable completed Solution record for the Explore archive workspace."""
        row = self.db.execute(
            """SELECT f.*, COALESCE(c.evidence, '') AS completion_evidence,
                      COALESCE(c.report, '') AS completion_report,
                      COALESCE(c.created_at, '') AS completed_at,
                      p.id AS problem_id,p.statement AS problem_statement,p.state AS problem_state,
                      COALESCE(cp.path, '') AS completion_playbook_path
               FROM features f
               JOIN problems p ON p.id=f.problem_id
               LEFT JOIN completions c ON c.feature_id=f.id
               LEFT JOIN completion_playbooks cp ON cp.problem_id=p.id
               WHERE f.id=? AND (c.state='verified' OR cp.path IS NOT NULL OR p.state='completed')""",
            (feature_id,),
        ).fetchone()
        if not row:
            raise WorkflowError("Completed Solution not found")
        solution = self.localized.overlay("features", row, locale)
        problem = self.db.execute("SELECT * FROM problems WHERE id=?", (row["problem_id"],)).fetchone()
        if problem:
            solution["problem_statement"] = self.localized.overlay("problems", problem, locale)["statement"]
        return solution

    def create_follow_up_problem(self, feature_id: str) -> dict[str, str]:
        """Create an explicit new Problem linked by text to an immutable completed Solution."""
        solution = self.completed_solution(feature_id)
        title = str(solution["title"])
        capture_id = self.capture(f"Follow up from completed Solution: {title}")
        detail = (
            "## Source completed Solution\n"
            f"{title}\n\n"
            "## Prior outcome\n"
            f"{solution['outcome']}\n\n"
            "## Follow-up context\n"
            "Created explicitly from the completed record. The new need is Not yet known."
        )
        problem = self.promote_capture(capture_id, f"Follow up: {title}", detail)
        self.db.execute(
            "INSERT INTO follow_up_links(problem_id,source_feature_id) VALUES (?,?)",
            (problem["id"], feature_id),
        )
        self.db.commit()
        return problem

    @staticmethod
    def _refinement_sections(value: str) -> dict[str, str]:
        """Extract lightweight Markdown/plain-text sections without asking AI to grade itself."""
        sections: dict[str, list[str]] = {}
        current = ""
        known = {
            "context", "background", "impact", "evidence", "desired outcome", "intended outcome",
            "boundaries", "scope", "non-goals", "non goals", "evidence and prior context",
            "trade-offs", "tradeoffs", "dependencies", "validation criteria", "risks", "open questions",
            "맥락", "배경", "영향", "근거", "증거", "목표", "의도한 결과", "범위", "제외 범위",
            "트레이드오프", "의존성", "검증 기준", "위험", "미결 질문",
        }
        for raw_line in str(value or "").splitlines():
            line = raw_line.strip()
            heading = re.match(r"^(?:#{1,6}\s*)?(?:\*\*)?([^:#*]{2,48})(?:\*\*)?\s*:?\s*$", line)
            name = re.sub(r"\s+", " ", heading.group(1)).strip().casefold() if heading else ""
            formatted = line.startswith("#") or (line.startswith("**") and line.rstrip(":").endswith("**")) or line.endswith(":")
            if heading and len(line.split()) <= 7 and (formatted or name in known):
                current = name
                sections.setdefault(current, [])
            elif current and line:
                sections[current].append(line)
        return {name: " ".join(lines).strip() for name, lines in sections.items()}

    def refinement_structure_assessment(self, entity_type: str, entity_id: str) -> dict[str, object]:
        """Describe readiness using the same information shape shown in final Item Detail."""
        if entity_type not in {"problems", "features"}:
            raise WorkflowError("Refinement structure supports Problem and Solution items only")
        table = "problems" if entity_type == "problems" else "features"
        row = self.db.execute(f"SELECT * FROM {table} WHERE id=?", (entity_id,)).fetchone()
        if not row:
            raise WorkflowError("Item not found")
        item = dict(row)
        detail = str(item.get("detail") or item.get("outcome") or "")
        sections = self._refinement_sections(detail)
        placeholder = re.compile(r"^(?:n/?a|none|not yet known|unknown|tbd|to be determined)[.!]?$", re.I)

        sources = self._context_lineage_sources(entity_type, entity_id)
        source_clause = " OR ".join("(entity_type=? AND entity_id=?)" for _ in sources)
        source_values = tuple(value for source in sources for value in source[:2])
        refinement_runs = self.db.execute(
            f"""SELECT kind,input_text,output_text FROM ai_runs
                WHERE ({source_clause}) AND kind IN ('workflow_chat','workflow_refinement')
                ORDER BY created_at,rowid""",
            source_values,
        ).fetchall()
        user_context = " ".join(str(run[1] or "") for run in refinement_runs if run[0] == "workflow_chat")

        def section_value(*aliases: str) -> str:
            for name, value in sections.items():
                if any(alias in name for alias in aliases):
                    return value
            return ""

        def field(
            key: str,
            label: str,
            value: str = "",
            *,
            threshold: int = 36,
            aliases: tuple[str, ...] = (),
            unknown_is_content: bool = False,
        ) -> dict[str, str]:
            cleaned = self._refinement_context_text(value)
            if cleaned and not placeholder.match(cleaned):
                contains_unknown = bool(re.search(r"\b(?:not yet known|unknown|tbd|to be determined)\b", cleaned, re.I))
                status = "complete" if len(cleaned) >= threshold and (unknown_is_content or not contains_unknown) else "weak"
                return {"key": key, "label": label, "status": status, "text": cleaned[:220]}
            matched = ""
            if aliases and user_context:
                sentences = re.split(r"(?<=[.!?。！？])\s+|\n+", user_context)
                matched = next((sentence.strip() for sentence in sentences if any(alias.casefold() in sentence.casefold() for alias in aliases)), "")
            if matched:
                return {"key": key, "label": label, "status": "weak", "text": self._refinement_context_text(matched)[:220]}
            return {"key": key, "label": label, "status": "missing", "text": "Not yet known"}

        if entity_type == "problems":
            structure = [
                field("statement", "Problem statement", str(item.get("statement") or ""), threshold=8),
                field("context", "Context", section_value("context", "background", "맥락", "배경") or (detail if not sections else ""), aliases=("context", "background", "맥락", "배경")),
                field("impact", "Impact", section_value("impact", "영향"), aliases=("impact", "affected", "cost", "영향", "불편", "손실")),
                field("evidence", "Evidence", section_value("evidence", "근거", "증거"), aliases=("evidence", "observed", "data", "근거", "증거", "피드백")),
                field("desired_outcome", "Desired outcome", section_value("desired outcome", "outcome", "목표", "결과"), aliases=("outcome", "success", "결과", "목표", "성공")),
                field("boundaries", "Boundaries", section_value("boundar", "non-goal", "scope", "범위", "제외"), aliases=("boundary", "scope", "non-goal", "범위", "제외")),
                field("open_questions", "Open questions", section_value("open question", "미결", "질문"), aliases=("unknown", "question", "미정", "질문"), unknown_is_content=True),
            ]
            priority = ["evidence", "desired_outcome", "impact", "boundaries", "context", "open_questions"]
            bundles = [{"impact", "evidence"}, {"desired_outcome", "boundaries"}]
        else:
            problem = self.db.execute("SELECT statement FROM problems WHERE id=?", (item.get("problem_id"),)).fetchone()
            raw_outcome = str(item.get("outcome") or "")
            intended = section_value("intended outcome", "desired outcome", "의도", "목표") or (raw_outcome if not sections else "")
            structure = [
                field("title", "Solution title", str(item.get("title") or ""), threshold=8),
                field("problem", "Problem this supports", str(problem[0]) if problem else "", threshold=8),
                field("intended_outcome", "Intended outcome", intended, aliases=("outcome", "success", "결과", "목표", "성공")),
                field("scope", "Scope", section_value("scope", "범위"), aliases=("scope", "include", "범위", "포함")),
                field("non_goals", "Non-goals", str(item.get("non_goals") or "") or section_value("non-goal", "제외"), aliases=("non-goal", "out of scope", "제외", "하지 않")),
                field("evidence", "Evidence & prior context", section_value("evidence", "prior context", "근거", "증거"), aliases=("evidence", "feedback", "data", "근거", "증거", "피드백")),
                field("tradeoffs_risks", "Trade-offs & risks", section_value("trade-off", "tradeoff", "risk", "위험", "트레이드"), aliases=("trade-off", "risk", "cost", "위험", "부작용")),
                field("dependencies", "Dependencies", section_value("dependenc", "의존"), aliases=("depend", "blocked", "의존", "선행")),
                field("validation_criteria", "Validation criteria", str(item.get("validation_criteria") or "") or section_value("validation", "acceptance", "검증"), aliases=("validate", "criterion", "measure", "검증", "확인", "측정")),
                field("open_questions", "Open questions", section_value("open question", "미결", "질문"), aliases=("unknown", "question", "미정", "질문"), unknown_is_content=True),
            ]
            priority = ["intended_outcome", "validation_criteria", "scope", "non_goals", "tradeoffs_risks", "dependencies", "evidence", "open_questions"]
            bundles = [{"intended_outcome", "validation_criteria"}, {"scope", "non_goals"}, {"tradeoffs_risks", "dependencies"}]

        by_key = {item["key"]: item for item in structure}
        # Field importance wins over cosmetic completeness: a thin intended outcome is
        # more important than a wholly missing low-priority open question.
        incomplete = [key for key in priority if by_key[key]["status"] != "complete"]
        focus_keys: list[str] = []
        if incomplete:
            first = incomplete[0]
            bundle = next((group for group in bundles if first in group), {first})
            focus_keys = [key for key in priority if key in bundle and by_key[key]["status"] != "complete"][:3] or [first]
        chat_count = sum(1 for run in refinement_runs if run[0] == "workflow_chat")
        refinement_count = sum(1 for run in refinement_runs if run[0] == "workflow_refinement")
        meaningful_sections = sum(1 for value in sections.values() if value and not placeholder.match(value))
        view_mode = "structure" if chat_count >= 2 or refinement_count or meaningful_sections >= 2 else "context"
        counts = {status: sum(1 for item in structure if item["status"] == status) for status in ("complete", "weak", "missing")}
        return {
            "view_mode": view_mode,
            "structure": structure,
            "focus": [{"key": key, "label": by_key[key]["label"], "status": by_key[key]["status"]} for key in focus_keys],
            "readiness": counts,
        }

    def assess_importance(self, problem_id: str, alignment: int, impact: int, urgency: int, leverage: int, evidence: str) -> dict[str, object]:
        if not self.db.execute("SELECT 1 FROM problems WHERE id=?", (problem_id,)).fetchone():
            raise WorkflowError("Problem not found")
        if not all(0 <= factor <= 5 for factor in (alignment, impact, urgency, leverage)) or not evidence.strip():
            raise WorkflowError("Each 0–5 importance factor requires evidence")
        importance = round(20 * (.35 * alignment + .30 * impact + .20 * urgency + .15 * leverage))
        self.db.execute("INSERT OR REPLACE INTO importance_assessments(id,problem_id,alignment,impact,urgency,leverage,evidence,importance) VALUES (?,?,?,?,?,?,?,?)", (str(uuid.uuid4()), problem_id, alignment, impact, urgency, leverage, evidence, importance))
        self.db.commit()
        return {"importance": importance, "evidence": evidence}

    def create_goal(self, title: str, description: str = "") -> dict[str, str]:
        goal_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO compass_goals(id,title,description) VALUES (?,?,?)", (goal_id, title, description))
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM compass_goals WHERE id=?", (goal_id,)).fetchone())

    def award(self, entity_type: str, entity_id: str, points: float, event_type: str, goal_id: str | None = None) -> None:
        self.db.execute("INSERT INTO score_events(id,entity_type,entity_id,goal_id,points,event_type) VALUES (?,?,?,?,?,?)", (str(uuid.uuid4()), entity_type, entity_id, goal_id, points, event_type))
        self._refresh_periods()
        self.db.commit()

    def _refresh_periods(self) -> None:
        self.db.execute("DELETE FROM score_periods")
        self.db.execute("""INSERT INTO score_periods(period,goal_id,points)
          SELECT substr(created_at,1,10), goal_id, sum(points) FROM score_events GROUP BY substr(created_at,1,10),goal_id""")

    def dashboard(self) -> dict[str, object]:
        return {
            "goals": [dict(row) for row in self.db.execute("SELECT * FROM compass_goals WHERE active=1 ORDER BY created_at")],
            "scores": [dict(row) for row in self.db.execute("SELECT * FROM score_periods ORDER BY period DESC")],
            "events": [dict(row) for row in self.db.execute("SELECT * FROM score_events ORDER BY created_at DESC LIMIT 50")],
        }

    def record_completion(self, feature_id: str, evidence: str, report: str, no_update_reason: str = "") -> dict[str, str]:
        feature = self.db.execute("SELECT state FROM features WHERE id=?", (feature_id,)).fetchone()
        if not feature or feature[0] != "approved":
            raise WorkflowError("Completion requires an approved feature")
        if not evidence.strip() or not report.strip():
            raise WorkflowError("Completion needs implementation evidence and a report")
        status = "not_needed" if no_update_reason.strip() else "pending"
        completion_id = str(uuid.uuid4())
        self.db.execute("INSERT OR REPLACE INTO completions(id,feature_id,evidence,report,knowledge_status,no_update_reason) VALUES (?,?,?,?,?,?)", (completion_id, feature_id, evidence, report, status, no_update_reason))
        self._record_solution_decision(
            feature_id,
            "completed",
            {"state": "approved"},
            {"completion_id": completion_id, "knowledge_status": status},
            report,
            "completion",
            completion_id,
        )
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM completions WHERE feature_id=?", (feature_id,)).fetchone())

    def verify_completion(self, feature_id: str) -> None:
        completion = self.db.execute("SELECT knowledge_status FROM completions WHERE feature_id=?", (feature_id,)).fetchone()
        if not completion or completion[0] not in {"integrated", "not_needed"}:
            raise WorkflowError("Completion needs approved knowledge integration or an explicit no-update reason")
        self.db.execute("UPDATE completions SET state='verified' WHERE feature_id=?", (feature_id,))
        importance = self.db.execute("SELECT i.importance FROM importance_assessments i JOIN features f ON f.problem_id=i.problem_id WHERE f.id=?", (feature_id,)).fetchone()
        if importance:
            self.award("feature", feature_id, float(importance[0]) * .70, "implementation_verified_and_integrated")
        self._approval("completion", feature_id, "verified")

    def save_completion_review(self, feature_id: str, report: object) -> str:
        if not self.db.execute("SELECT 1 FROM features WHERE id=?", (feature_id,)).fetchone():
            raise WorkflowError("Solution not found")
        review_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO completion_reviews(id,feature_id,report_json) VALUES (?,?,?)", (review_id, feature_id, __import__("json").dumps(report)))
        self.db.commit()
        return review_id

    def complete_problem(self, problem_id: str, reason: str = "", review_id: str = "") -> None:
        if not self.db.execute("SELECT 1 FROM problems WHERE id=?", (problem_id,)).fetchone():
            raise WorkflowError("Problem not found")
        self.db.execute("UPDATE problems SET state='completed' WHERE id=?", (problem_id,))
        self.db.execute(
            "INSERT INTO problem_completion_decisions(id,problem_id,review_id,reason) VALUES (?,?,?,?)",
            (str(uuid.uuid4()), problem_id, review_id, reason.strip()),
        )
        self._approval("problem", problem_id, "completion_confirmed")
        self.db.commit()

    def create_lineage_snapshot(self, feature_id: str, force: bool = False) -> dict[str, object]:
        try:
            document, source_hash = build_lineage_document(self.db, feature_id)
        except ValueError as error:
            raise WorkflowError(str(error)) from error
        existing = self.db.execute(
            """SELECT id FROM lineage_snapshots
               WHERE feature_id=? AND source_hash=? AND schema_version=?
               ORDER BY version DESC LIMIT 1""",
            (feature_id, source_hash, LINEAGE_SCHEMA_VERSION),
        ).fetchone()
        if existing and not force:
            return self.lineage(feature_id, str(existing["id"]))

        prior_corrections: dict[str, dict[str, object]] = {}
        for row in self.db.execute(
            """SELECT lc.claim_key,lr.* FROM lineage_claims lc
               JOIN lineage_snapshots ls ON ls.id=lc.snapshot_id
               JOIN lineage_revisions lr ON lr.claim_id=lc.id AND lr.is_current=1 AND lr.author_type='user'
               WHERE ls.feature_id=? ORDER BY ls.version DESC""",
            (feature_id,),
        ).fetchall():
            prior_corrections.setdefault(str(row["claim_key"]), dict(row))
        version_row = self.db.execute(
            "SELECT COALESCE(MAX(version),0)+1 FROM lineage_snapshots WHERE feature_id=?", (feature_id,)
        ).fetchone()
        version = int(version_row[0])
        snapshot_id = str(uuid.uuid4())
        evidence_by_key = {str(item["key"]): item for item in document.pop("evidence", [])}
        claim_id_by_key: dict[str, str] = {}
        claims_payload: dict[str, dict[str, object]] = {}
        evidence_payload: dict[str, dict[str, object]] = {}

        self.db.execute(
            """INSERT INTO lineage_snapshots(id,feature_id,version,schema_version,source_hash,status,document_json)
               VALUES (?,?,?,?,?,'building','{}')""",
            (snapshot_id, feature_id, version, LINEAGE_SCHEMA_VERSION, source_hash),
        )
        for claim in document.pop("claims", []):
            claim_id = str(uuid.uuid4())
            claim_key = str(claim["claim_key"])
            claim_id_by_key[claim_key] = claim_id
            self.db.execute(
                """INSERT INTO lineage_claims(
                     id,snapshot_id,claim_key,section,subject_type,subject_id,classification,confidence,material
                   ) VALUES (?,?,?,?,?,?,?,?,?)""",
                (
                    claim_id,
                    snapshot_id,
                    claim_key,
                    claim["section"],
                    claim["subject_type"],
                    claim["subject_id"],
                    claim["classification"],
                    claim.get("confidence"),
                    int(bool(claim.get("material"))),
                ),
            )
            initial_revision = str(uuid.uuid4())
            carried = prior_corrections.get(claim_key)
            self.db.execute(
                """INSERT INTO lineage_revisions(id,claim_id,author_type,text,is_current)
                   VALUES (?,?,?,?,?)""",
                (initial_revision, claim_id, "ai" if claim["classification"] == "inferred" else "deterministic", claim["text"], 0 if carried else 1),
            )
            current_revision = initial_revision
            current_text = str(claim["text"])
            author_type = "ai" if claim["classification"] == "inferred" else "deterministic"
            if carried:
                current_revision = str(uuid.uuid4())
                current_text = str(carried["text"])
                author_type = "user"
                self.db.execute(
                    """INSERT INTO lineage_revisions(id,claim_id,supersedes_id,author_type,text,reason,is_current)
                       VALUES (?,?,?,?,?,?,1)""",
                    (current_revision, claim_id, initial_revision, "user", current_text, str(carried["reason"] or "")),
                )
            evidence_ids: list[str] = []
            for evidence_key in claim.get("evidence_keys", []):
                source = evidence_by_key[str(evidence_key)]
                evidence_id = str(uuid.uuid4())
                evidence_ids.append(evidence_id)
                live = source.get("live_record") or {}
                self.db.execute(
                    """INSERT INTO lineage_evidence(
                         id,claim_id,source_type,source_id,field_name,excerpt,source_hash,live_entity_type
                       ) VALUES (?,?,?,?,?,?,?,?)""",
                    (
                        evidence_id,
                        claim_id,
                        source["source_type"],
                        source["source_id"],
                        source["field_name"],
                        source["excerpt"],
                        source["source_hash"],
                        str(live.get("entity_type") or ""),
                    ),
                )
                evidence_payload[evidence_id] = {
                    "id": evidence_id,
                    **{key: value for key, value in source.items() if key not in {"key", "live_record"}},
                    "live_record": live or None,
                }
            claims_payload[claim_id] = {
                "id": claim_id,
                "claim_key": claim_key,
                "section": claim["section"],
                "classification": claim["classification"],
                "confidence": claim.get("confidence"),
                "material": bool(claim.get("material")),
                "text": current_text,
                "evidence_ids": evidence_ids,
                "current_revision_id": current_revision,
                "current_author_type": author_type,
                "revisions": 2 if carried else 1,
            }

        for stage in document["lineage"]["stages"]:
            stage["claim_id"] = claim_id_by_key.pop(stage.pop("claim_key"))
        for transition in document["lineage"]["transitions"]:
            transition["claim_id"] = claim_id_by_key.pop(transition.pop("claim_key"))
        for collection in ("decision_changes", "conflicts", "completion_evidence"):
            for item in document[collection]:
                item["claim_id"] = claim_id_by_key.pop(item.pop("claim_key"))
        document.update({
            "snapshot_id": snapshot_id,
            "feature_id": feature_id,
            "version": version,
            "status": "ready_without_inference",
            "claims": claims_payload,
            "evidence": evidence_payload,
            "generation": {"schema_version": LINEAGE_SCHEMA_VERSION, "inference_error": ""},
        })
        self.db.execute(
            "UPDATE lineage_snapshots SET status='ready_without_inference',document_json=? WHERE id=?",
            (json.dumps(document, ensure_ascii=False, sort_keys=True), snapshot_id),
        )
        self.db.commit()
        return self.lineage(feature_id, snapshot_id)

    def lineage(self, feature_id: str, snapshot_id: str = "") -> dict[str, object]:
        if snapshot_id:
            snapshot = self.db.execute(
                "SELECT * FROM lineage_snapshots WHERE id=? AND feature_id=?", (snapshot_id, feature_id)
            ).fetchone()
        else:
            snapshot = self.db.execute(
                "SELECT * FROM lineage_snapshots WHERE feature_id=? ORDER BY version DESC LIMIT 1", (feature_id,)
            ).fetchone()
        if not snapshot:
            raise WorkflowError("Lineage snapshot not found")
        document = json.loads(snapshot["document_json"] or "{}")
        stage_tables = {"capture": "captures", "problem": "problems", "solution": "features"}
        for stage in document.get("lineage", {}).get("stages", []):
            table = stage_tables.get(str(stage.get("kind")))
            if table:
                stage.setdefault("record_type", table)
                if not stage.get("occurred_at") and stage.get("record_id"):
                    source = self.db.execute(
                        f"SELECT created_at FROM {table} WHERE id=?", (stage["record_id"],)
                    ).fetchone()
                    if source:
                        stage["occurred_at"] = source["created_at"]
            elif stage.get("kind") == "complete" and not stage.get("occurred_at"):
                completed = self.db.execute(
                    """SELECT pcd.created_at FROM problem_completion_decisions pcd
                       JOIN features f ON f.problem_id=pcd.problem_id
                       WHERE f.id=? ORDER BY pcd.created_at DESC,pcd.rowid DESC LIMIT 1""",
                    (feature_id,),
                ).fetchone()
                if completed:
                    stage["occurred_at"] = completed["created_at"]
        claims: dict[str, dict[str, object]] = {}
        evidence: dict[str, dict[str, object]] = {}
        for claim_row in self.db.execute("SELECT * FROM lineage_claims WHERE snapshot_id=? ORDER BY rowid", (snapshot["id"],)).fetchall():
            claim = dict(claim_row)
            revisions = [dict(row) for row in self.db.execute(
                "SELECT * FROM lineage_revisions WHERE claim_id=? ORDER BY created_at,rowid", (claim["id"],)
            ).fetchall()]
            current = next((item for item in revisions if item["is_current"]), revisions[-1])
            evidence_ids: list[str] = []
            for item in self.db.execute("SELECT * FROM lineage_evidence WHERE claim_id=? ORDER BY rowid", (claim["id"],)).fetchall():
                value = dict(item)
                evidence_ids.append(value["id"])
                live_available = False
                if value["live_entity_type"] in {"captures", "problems", "features"}:
                    live_available = bool(self.db.execute(
                        f"SELECT 1 FROM {value['live_entity_type']} WHERE id=?", (value["source_id"],)
                    ).fetchone())
                value["live_record"] = {
                    "available": live_available,
                    "entity_type": value["live_entity_type"],
                    "entity_id": value["source_id"],
                } if value["live_entity_type"] else None
                evidence[value["id"]] = value
            claims[claim["id"]] = {
                **claim,
                "material": bool(claim["material"]),
                "text": current["text"],
                "current_revision_id": current["id"],
                "current_author_type": current["author_type"],
                "evidence_ids": evidence_ids,
                "revisions": revisions,
            }
        document.update({
            "snapshot_id": snapshot["id"],
            "feature_id": feature_id,
            "version": snapshot["version"],
            "status": snapshot["status"],
            "source_hash": snapshot["source_hash"],
            "claims": claims,
            "evidence": evidence,
            "generation": {
                "schema_version": snapshot["schema_version"],
                "created_at": snapshot["created_at"],
                "inference_error": snapshot["inference_error"],
            },
        })
        return document

    def lineages_for_problem(self, problem_id: str, ensure: bool = True) -> list[dict[str, object]]:
        feature_ids = [str(row[0]) for row in self.db.execute(
            "SELECT id FROM features WHERE problem_id=? ORDER BY created_at,rowid", (problem_id,)
        ).fetchall()]
        result: list[dict[str, object]] = []
        for feature_id in feature_ids:
            try:
                result.append(self.lineage(feature_id))
            except WorkflowError:
                if ensure:
                    result.append(self.create_lineage_snapshot(feature_id))
        return result

    def lineage_evidence(self, feature_id: str, evidence_id: str) -> dict[str, object]:
        lineage = self.lineage(feature_id)
        evidence = lineage["evidence"].get(evidence_id)
        if not evidence:
            raise WorkflowError("Lineage evidence not found")
        return evidence

    def correct_lineage_claim(
        self,
        feature_id: str,
        claim_id: str,
        text: str,
        reason: str = "",
        current_revision_id: str = "",
    ) -> dict[str, object]:
        if not text.strip():
            raise WorkflowError("Lineage correction cannot be empty")
        claim = self.db.execute(
            """SELECT lc.* FROM lineage_claims lc JOIN lineage_snapshots ls ON ls.id=lc.snapshot_id
               WHERE lc.id=? AND ls.feature_id=? ORDER BY ls.version DESC LIMIT 1""",
            (claim_id, feature_id),
        ).fetchone()
        if not claim:
            raise WorkflowError("Lineage claim not found")
        if claim["classification"] != "inferred":
            raise WorkflowError("Only AI interpretations can be corrected; source-backed records are immutable")
        current = self.db.execute(
            "SELECT * FROM lineage_revisions WHERE claim_id=? AND is_current=1", (claim_id,)
        ).fetchone()
        if not current:
            raise WorkflowError("Current lineage revision not found")
        if current_revision_id and current["id"] != current_revision_id:
            raise WorkflowError("Lineage claim changed; reload before correcting")
        revision_id = str(uuid.uuid4())
        self.db.execute("UPDATE lineage_revisions SET is_current=0 WHERE claim_id=? AND is_current=1", (claim_id,))
        self.db.execute(
            """INSERT INTO lineage_revisions(id,claim_id,supersedes_id,author_type,text,reason,is_current)
               VALUES (?,?,?,?,?,?,1)""",
            (revision_id, claim_id, current["id"], "user", text.strip(), reason.strip()),
        )
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM lineage_revisions WHERE id=?", (revision_id,)).fetchone())

    def add_lineage_inferences(self, feature_id: str, snapshot_id: str, inferences: list[dict[str, object]]) -> dict[str, object]:
        snapshot = self.db.execute(
            "SELECT * FROM lineage_snapshots WHERE id=? AND feature_id=?", (snapshot_id, feature_id)
        ).fetchone()
        if not snapshot:
            raise WorkflowError("Lineage snapshot not found")
        document = json.loads(snapshot["document_json"])
        evidence_ids = {
            str(row[0]) for row in self.db.execute(
                "SELECT le.id FROM lineage_evidence le JOIN lineage_claims lc ON lc.id=le.claim_id WHERE lc.snapshot_id=?",
                (snapshot_id,),
            )
        }
        for inference in inferences:
            cited = [str(item) for item in inference["evidence_ids"]]
            if any(item not in evidence_ids for item in cited):
                raise WorkflowError("Lineage inference cited unknown evidence")
            claim_id = str(uuid.uuid4())
            claim_key = str(inference["claim_key"])
            carried = self.db.execute(
                """SELECT lr.* FROM lineage_claims lc
                   JOIN lineage_snapshots ls ON ls.id=lc.snapshot_id
                   JOIN lineage_revisions lr ON lr.claim_id=lc.id AND lr.is_current=1 AND lr.author_type='user'
                   WHERE ls.feature_id=? AND lc.claim_key=? AND ls.id<>?
                   ORDER BY ls.version DESC LIMIT 1""",
                (feature_id, claim_key, snapshot_id),
            ).fetchone()
            self.db.execute(
                """INSERT INTO lineage_claims(
                     id,snapshot_id,claim_key,section,subject_type,subject_id,classification,confidence,material
                   ) VALUES (?,?,?,?,? ,?,'inferred',?,0)""",
                (claim_id, snapshot_id, claim_key, "decision_change", "solution", feature_id, inference["confidence"]),
            )
            revision_id = str(uuid.uuid4())
            self.db.execute(
                "INSERT INTO lineage_revisions(id,claim_id,author_type,text,is_current) VALUES (?,?, 'ai',?,?)",
                (revision_id, claim_id, inference["text"], 0 if carried else 1),
            )
            if carried:
                carried_id = str(uuid.uuid4())
                self.db.execute(
                    """INSERT INTO lineage_revisions(id,claim_id,supersedes_id,author_type,text,reason,is_current)
                       VALUES (?,?,?,?,?,?,1)""",
                    (carried_id, claim_id, revision_id, "user", carried["text"], carried["reason"]),
                )
            for evidence_id in cited:
                source = self.db.execute("SELECT * FROM lineage_evidence WHERE id=?", (evidence_id,)).fetchone()
                clone_id = str(uuid.uuid4())
                self.db.execute(
                    """INSERT INTO lineage_evidence(
                         id,claim_id,source_type,source_id,field_name,excerpt,source_hash,live_entity_type,captured_at
                       ) VALUES (?,?,?,?,?,?,?,?,?)""",
                    (clone_id, claim_id, source["source_type"], source["source_id"], source["field_name"], source["excerpt"], source["source_hash"], source["live_entity_type"], source["captured_at"]),
                )
            document.setdefault("decision_changes", []).append({"claim_id": claim_id, "event_type": "ai_inferred"})
        document["status"] = "ready"
        self.db.execute(
            "UPDATE lineage_snapshots SET status='ready',document_json=?,inference_error='' WHERE id=?",
            (json.dumps(document, ensure_ascii=False, sort_keys=True), snapshot_id),
        )
        self.db.commit()
        return self.lineage(feature_id, snapshot_id)

    def set_lineage_inference_error(self, feature_id: str, snapshot_id: str, error: str) -> dict[str, object]:
        self.db.execute(
            "UPDATE lineage_snapshots SET status='ready_without_inference',inference_error=? WHERE id=? AND feature_id=?",
            (error[:1000], snapshot_id, feature_id),
        )
        self.db.commit()
        return self.lineage(feature_id, snapshot_id)

    def mark_lineage_inference_complete(self, feature_id: str, snapshot_id: str) -> dict[str, object]:
        self.db.execute(
            "UPDATE lineage_snapshots SET status='ready',inference_error='' WHERE id=? AND feature_id=?",
            (snapshot_id, feature_id),
        )
        self.db.commit()
        return self.lineage(feature_id, snapshot_id)

    def completion_playbook(
        self,
        problem_id: str,
        directory: str,
        raw: bool = False,
        executive_summary: str = "",
        report_body: str = "",
        lineages: list[dict[str, object]] | None = None,
    ) -> tuple[str, str]:
        """Render a dense, deterministic completion record for vault-first reuse."""
        problem = self.db.execute("SELECT * FROM problems WHERE id=?", (problem_id,)).fetchone()
        if not problem:
            raise WorkflowError("Problem not found")
        problem = self.localized.overlay("problems", problem, "en")
        features = self.localized.overlay_many(
            "features",
            self.db.execute("SELECT * FROM features WHERE problem_id=? ORDER BY created_at", (problem_id,)).fetchall(),
            "en",
        )
        decisions = self.db.execute("SELECT * FROM problem_completion_decisions WHERE problem_id=? ORDER BY created_at", (problem_id,)).fetchall()
        run_rows = self.db.execute("SELECT * FROM ai_runs WHERE entity_id IN (SELECT id FROM features WHERE problem_id=?) OR entity_id=? ORDER BY created_at", (problem_id, problem_id)).fetchall()
        reviews = self.db.execute("SELECT cr.* FROM completion_reviews cr JOIN features f ON f.id=cr.feature_id WHERE f.problem_id=? ORDER BY cr.created_at", (problem_id,)).fetchall()
        conflict_rows = self.db.execute("SELECT c.* FROM conflict_reports c JOIN features f ON f.id=c.feature_id WHERE f.problem_id=? ORDER BY c.created_at", (problem_id,)).fetchall()
        # Archive names are read by people, so remove workflow-state prefixes and
        # never expose the database UUID in the document name or frontmatter.
        title = str(problem["statement"]).replace("\n", " ").strip()[:90] or "Completed work"
        title = re.sub(r"^(?:(?:in[ -]?progress|proposed|completed)\s+)?(?:solution|problem)\s*[:\-–—]*\s*", "", title, flags=re.IGNORECASE).strip() or "Completed work"
        safe_title = "".join(char if char.isalnum() or char in " -_" else "" for char in title).strip() or problem_id
        path = f"{directory.strip('/')}/{safe_title}.md"
        raw_path = f"{directory.strip('/')}/assets/{safe_title}.raw.md"
        summary = [
            f"Problem: {problem['statement']}",
            "Status: completed by explicit human decision.",
            f"Solutions recorded: {len(features)}; progress records are retained with each Solution.",
            "Read this Summary first; use the linked sections below for evidence, decisions, and the reusable playbook.",
            "Image summaries recorded during work are the canonical visual summary at completion; do not create a second completion summary for those captures.",
        ]
        lines = [
            "---", "type: completed-work-playbook", "status: completed", "llm_wiki_managed: true", "canonical_locale: en",
            "tags: [llm-wiki, completed-work, playbook]", "---", "", f"# {title}", "", "## Summary", *[f"- {item}" for item in summary],
            "", "## Problem context", str(problem["detail"] or problem["statement"]), "", "## Approved solutions and decisions",
        ]
        for feature in features:
            lines.extend([f"### {feature['title']}", f"- Intended outcome: {feature['outcome']}", f"- Non-goals: {feature['non_goals'] or 'None recorded.'}", "- Validation Criteria:", feature["validation_criteria"] or "Not recorded.", f"- Conflict state: {feature['conflict_state']}", f"- Workflow state: {feature['state']}", ""])
        lines.extend(["## Solution progress records"])
        for feature in features:
            progress = self.solution_progress(feature["id"])
            lines.extend([f"### {feature['title']}"])
            for entry in progress["entries"]:
                lines.extend([f"- {entry['created_at']}: {entry['body'] or 'Image capture recorded.'}"])
                if entry["image_data"]:
                    media_type = str(entry["image_media_type"] or "image/png")
                    extension = {"image/png": "png", "image/jpeg": "jpg", "image/webp": "webp", "image/gif": "gif"}.get(media_type, "bin")
                    # Raw Markdown lives beside these image files in assets/.
                    # A filename-only Obsidian embed avoids resolving assets/
                    # twice from an already nested Raw document.
                    asset_name = f"{entry['id']}.{extension}"
                    lines.extend([f"  - Original capture: ![[{asset_name}]]"])
                if entry["image_summary"]:
                    lines.extend([f"  - Canonical AI image summary: {entry['image_summary']}"])
                for comment in entry["comments"]:
                    lines.extend([f"  - Comment · {comment['created_at']}: {comment['body']}"])
            for item in progress["checklist"]:
                lines.extend([f"- [{'x' if item['checked'] else ' '}] {item['body']}"])
            lines.append("")
        lines.extend(["## Conflict analysis history"])
        for row in conflict_rows:
            lines.extend([f"- {row['state']}: {row['citation'] or 'Human-reviewed conflict decision.'}"])
        lines.extend(["", "## Completion review history"])
        for row in reviews:
            try:
                report = json.loads(row["report_json"])
            except (TypeError, json.JSONDecodeError):
                report = {"summary": row["report_json"]}
            lines.extend([f"- {report.get('resolution', 'review')}: {report.get('summary', 'No summary recorded.')}"])
        lines.extend(["", "## Human completion decision"])
        for row in decisions:
            lines.extend([f"- {row['created_at']}: {row['reason'] or 'Completed after human review; no additional reason recorded.'}"])
        lines.extend(["", "## Feedback and workflow history"])
        for row in run_rows:
            lines.extend([f"### {row['kind']} · {row['created_at']}", row["output_text"] or row["input_text"], ""])
        raw_content = "\n".join(lines)
        if raw:
            return raw_path, raw_content

        related: list[str] = []
        title_terms = {word.casefold() for word in re.findall(r"[\w-]{4,}", title)}
        for row in self.db.execute("SELECT path FROM completion_playbooks WHERE problem_id<>? ORDER BY created_at DESC", (problem_id,)):
            candidate = str(row["path"])
            candidate_terms = {word.casefold() for word in re.findall(r"[\w-]{4,}", candidate)}
            if len(title_terms & candidate_terms) >= 2:
                related.append(candidate)
            if len(related) == 3:
                break
        # Keep this document genuinely useful without inventing a retrospective.
        # Every sentence below is either a workflow field or a count from the
        # preserved record.  The full, unabridged material remains in Raw Data.
        def concise(value: object, limit: int = 360) -> str:
            text = re.sub(r"\s+", " ", str(value or "")).strip()
            text = re.sub(r"^[-*]\s*(?:\[[ xX]\]\s*)?", "", text)
            return text if len(text) <= limit else text[: limit - 1].rstrip() + "…"

        progress_counts = {"entries": 0, "comments": 0, "images": 0, "checklist": 0, "checked": 0}
        for feature in features:
            progress = self.solution_progress(feature["id"])
            progress_counts["entries"] += len(progress["entries"])
            progress_counts["comments"] += sum(len(entry["comments"]) for entry in progress["entries"])
            progress_counts["images"] += sum(1 for entry in progress["entries"] if entry["image_data"])
            progress_counts["checklist"] += len(progress["checklist"])
            progress_counts["checked"] += sum(1 for item in progress["checklist"] if item["checked"])

        main_lines = [
            "---", "type: completed-work-playbook", "status: completed", "llm_wiki_managed: true", "canonical_locale: en",
            "tags: [llm-wiki, completed-work, playbook]", "---", "", f"# {title}", "", "## Executive Summary",
            "", "### Purpose", concise(problem["statement"], 220), "", "### What was recorded for completion",
        ]
        problem_detail = concise(problem["detail"], 240)
        if problem_detail and problem_detail != concise(problem["statement"], 220):
            main_lines.extend(["", "### Context recorded", problem_detail])
        if features:
            for feature in features:
                main_lines.extend([
                    f"- **{concise(feature['title'], 100)}** — intended outcome: {concise(feature['outcome'], 220)}",
                    f"  - Workflow record: {feature['state']}; conflict record: {feature['conflict_state']}.",
                ])
        else:
            main_lines.append("- No Solution record was attached to this completed Problem.")

        main_lines.extend(["", "### Evidence retained"])
        evidence_bits = [
            f"{progress_counts['entries']} work log{'s' if progress_counts['entries'] != 1 else ''}",
            f"{progress_counts['comments']} comment{'s' if progress_counts['comments'] != 1 else ''}",
            f"{progress_counts['checklist']} checklist item{'s' if progress_counts['checklist'] != 1 else ''} ({progress_counts['checked']} checked)",
        ]
        if progress_counts["images"]:
            evidence_bits.append(f"{progress_counts['images']} original image capture{'s' if progress_counts['images'] != 1 else ''}")
        main_lines.append("- Retained: " + "; ".join(evidence_bits) + ".")
        if reviews:
            latest_review = reviews[-1]
            try:
                review = json.loads(latest_review["report_json"])
            except (TypeError, json.JSONDecodeError):
                review = {"summary": latest_review["report_json"]}
            main_lines.append(f"- Latest completion review: {concise(review.get('executive_summary') or review.get('summary') or review.get('resolution') or 'Recorded without a summary.')}.")
        else:
            main_lines.append("- No completion-review summary was recorded.")
        if decisions:
            latest_decision = decisions[-1]
            reason = concise(latest_decision["reason"], 180)
            main_lines.append(f"- Human completion decision ({latest_decision['created_at']}): {reason or 'No additional reason recorded.'}")

        main_lines.extend([
            "", "### Reuse note",
            "- Recorded AI Image Summaries are the canonical visual summaries; do not create a second completion-time image summary.",
        ])
        if executive_summary.strip():
            # The AI summary is a compact retrieval index; the narrative stays
            # in a separate report body so future conflict checks can read it first.
            main_lines = [
                "---", "type: completed-work-playbook", "status: completed", "llm_wiki_managed: true", "canonical_locale: en",
                "tags: [llm-wiki, completed-work, playbook]", "---", "", f"# {title}", "", "## Executive Summary", "", executive_summary.strip(),
            ]
            if report_body.strip():
                main_lines.extend(["", "## Completion Report", "", report_body.strip()])
        else:
            # Offline fallback: concise, factual, and deliberately separate
            # from the detailed Raw Data rather than copying it into a report.
            completed_outcomes = [f"- {concise(feature['title'], 100)}: {concise(feature['outcome'], 180)}" for feature in features]
            main_lines = [
                "---", "type: completed-work-playbook", "status: completed", "llm_wiki_managed: true", "canonical_locale: en",
                "tags: [llm-wiki, completed-work, playbook]", "---", "", f"# {title}", "", "## Executive Summary", "",
                "### Why", concise(problem["statement"], 220), "", "### What changed", *(completed_outcomes or ["- No Solution record was attached."]),
                "", "### How the work was carried out", f"- The preserved work record contains {progress_counts['entries']} work logs and {progress_counts['comments']} comments.",
                "", "### Final verification", f"- {progress_counts['checklist']} checklist items were recorded; {progress_counts['checked']} are checked.",
                "", "### Decision and risks", "- The human completion decision and any unresolved evidence are preserved in Raw Data.",
            ]
        for lineage in lineages or []:
            main_lines.extend(["", *render_lineage_markdown(lineage)])
        if related:
            main_lines.extend(["## Related Playbooks", *[f"- [[{item}]]" for item in related], ""])
        # CommonMark requires angle brackets around destinations containing
        # spaces; Obsidian otherwise truncates the link at the first space.
        main_lines.extend(["## Supporting evidence", f"- [Raw work record](<assets/{raw_path.rsplit('/', 1)[1]}>)", ""])
        return path, "\n".join(main_lines)

    def completion_assets(self, problem_id: str, directory: str) -> list[tuple[str, bytes]]:
        """Return immutable image captures that belong beside the completed-work Markdown."""
        rows = self.db.execute(
            "SELECT e.id,e.image_data,e.image_media_type FROM solution_progress_entries e JOIN features f ON f.id=e.feature_id WHERE f.problem_id=? AND e.image_data<>''",
            (problem_id,),
        ).fetchall()
        assets: list[tuple[str, bytes]] = []
        for row in rows:
            media_type = str(row["image_media_type"] or "image/png")
            extension = {"image/png": "png", "image/jpeg": "jpg", "image/webp": "webp", "image/gif": "gif"}.get(media_type, "bin")
            try:
                data = base64.b64decode(str(row["image_data"]), validate=True)
            except ValueError:
                continue
            assets.append((f"{directory.strip('/')}/assets/{row['id']}.{extension}", data))
        return assets

    def remember_completion_playbook(
        self,
        problem_id: str,
        path: str,
        source_hash: str,
        lineage_snapshot_id: str = "",
        lineage_version: int = 0,
        report_input_hash: str = "",
        report_generation_status: str = "deterministic_fallback",
    ) -> None:
        self.db.execute(
            """INSERT OR REPLACE INTO completion_playbooks(
                 problem_id,path,source_hash,lineage_snapshot_id,lineage_version,lineage_schema_version,
                 report_input_hash,report_generation_status
               ) VALUES (?,?,?,?,?,?,?,?)""",
            (
                problem_id,
                path,
                source_hash,
                lineage_snapshot_id,
                lineage_version,
                LINEAGE_SCHEMA_VERSION if lineage_snapshot_id else 0,
                report_input_hash,
                report_generation_status,
            ),
        )
        self.db.commit()

    def forget_completion_playbook(self, problem_id: str) -> None:
        self.db.execute("DELETE FROM completion_playbooks WHERE problem_id=?", (problem_id,))
        self.db.commit()

    def handoff(self, feature_id: str) -> str:
        feature = self.db.execute("SELECT f.*,p.statement FROM features f JOIN problems p ON p.id=f.problem_id WHERE f.id=?", (feature_id,)).fetchone()
        if not feature or feature["state"] != "approved":
            raise WorkflowError("Handoff requires an approved feature")
        lines = ["# Implementation handoff", "", "## Approved problem", feature["statement"], "", "## Approved solution", feature["title"], feature["outcome"], "", "## Constraints and non-goals", feature["non_goals"] or "None recorded.", "", "## Definition of done", "Review the intended outcome and validation criteria recorded in the Solution.", "", "## Unanswered questions", "None recorded."]
        return "\n".join(lines)

    def save_patch_proposal(self, feature_id: str, path: str, operation: str, heading: str, content: str, base_hash: str, before: str, proposed: str) -> dict[str, str]:
        patch_id = str(uuid.uuid4())
        self.db.execute("INSERT INTO patch_proposals(id,feature_id,path,operation,heading,content,base_hash,before_text,proposed_text) VALUES (?,?,?,?,?,?,?,?,?)", (patch_id, feature_id, path, operation, heading, content, base_hash, before, proposed))
        self.db.commit()
        return dict(self.db.execute("SELECT * FROM patch_proposals WHERE id=?", (patch_id,)).fetchone())

    def patch(self, patch_id: str) -> dict[str, str]:
        row = self.db.execute("SELECT * FROM patch_proposals WHERE id=?", (patch_id,)).fetchone()
        if not row:
            raise WorkflowError("Patch proposal not found")
        return dict(row)

    def mark_patch_applied(self, patch_id: str) -> None:
        patch = self.patch(patch_id)
        self.db.execute("UPDATE patch_proposals SET status='applied',reverse_text=? WHERE id=?", (patch["before_text"], patch_id))
        self.db.execute("UPDATE completions SET knowledge_status='integrated' WHERE feature_id=?", (patch["feature_id"],))
        self.db.commit()

    def undo_patch(self, patch_id: str) -> dict[str, str]:
        patch = self.patch(patch_id)
        if patch["status"] != "applied":
            raise WorkflowError("Only an applied patch can be undone")
        return patch

    def projection(self, entity_type: str, entity_id: str) -> tuple[str, str]:
        if entity_type not in {"questions", "problems", "features"}:
            raise WorkflowError("Unsupported projection type")
        singular = entity_type[:-1] if entity_type.endswith("s") else entity_type
        row = self.db.execute(f"SELECT * FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
        if not row:
            raise WorkflowError("Projection entity not found")
        values = dict(row)
        if entity_type in {"problems", "features"}:
            values = self.localized.overlay(entity_type, values, "en")
        title = values["statement"] if entity_type == "problems" else values.get("title", values.get("question", singular))
        safe_title = "".join(char if char.isalnum() or char in " -_" else "" for char in str(title)).strip()[:80] or entity_id
        number = {"questions": "10. Questions", "problems": "20. Problems", "features": "30. Features"}[entity_type]
        path = f"{date.today().year}/{number}/{safe_title}.md"
        frontmatter = f"---\nllm_wiki_id: {entity_id}\nllm_wiki_managed: true\ncanonical_locale: en\ntype: {singular}\nstate: {row['state']}\n---\n"
        metadata = {"content_locale", "available_locales", "fallback_used", "localized_versions"}
        body = "\n".join(f"- **{key.replace('_', ' ').title()}**: {value}" for key, value in values.items() if key not in {"id", "capture_id", "problem_id", "feature_id", "created_at", *metadata} and value)
        return path, f"{frontmatter}\n# {title}\n\n{body}\n"

    def mirror(self, entity_type: str, entity_id: str, path: str, source_hash: str) -> None:
        self.db.execute("INSERT OR REPLACE INTO mirror_files(entity_type,entity_id,path,source_hash) VALUES (?,?,?,?)", (entity_type, entity_id, path, source_hash))
        self.db.commit()

    def archive(self, entity_type: str, entity_id: str) -> str:
        if entity_type == "features":
            verified = self.db.execute("SELECT state FROM completions WHERE feature_id=?", (entity_id,)).fetchone()
            if not verified or verified[0] != "verified":
                raise WorkflowError("A feature can be archived only after verified completion")
        mirror = self.db.execute("SELECT path FROM mirror_files WHERE entity_type=? AND entity_id=?", (entity_type, entity_id)).fetchone()
        if not mirror:
            raise WorkflowError("Create a generated projection before archiving")
        self.db.execute(f"UPDATE {entity_type} SET state='archived' WHERE id=?", (entity_id,))
        self.db.commit()
        return str(mirror[0])

    def _approval(self, entity_type: str, entity_id: str, action: str) -> None:
        self.db.execute("INSERT INTO approvals(id,entity_type,entity_id,action) VALUES (?,?,?,?)", (str(uuid.uuid4()), entity_type, entity_id, action))
        self.db.commit()
