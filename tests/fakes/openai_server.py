from __future__ import annotations

import argparse
import json
import re
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Handler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        if self.path == "/v1/models":
            self._json({"data": [{"id": "deterministic-test-model"}]})
        else:
            self.send_error(404)

    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        request = json.loads(self.rfile.read(length) or b"{}")
        if self.path != "/v1/chat/completions":
            self.send_error(404)
            return
        if request.get("model") == "deterministic-failure":
            self.send_response(400)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"error":{"message":"deterministic provider failure"}}')
            return
        if request.get("model") == "deterministic-timeout":
            time.sleep(0.6)
        if request.get("stream"):
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for text in ("Deterministic ", "desktop response"):
                event = {"choices": [{"delta": {"content": text}}]}
                self.wfile.write(f"data: {json.dumps(event)}\n\n".encode())
                self.wfile.flush()
            self.wfile.write(b"data: [DONE]\n\n")
            return
        messages = request.get("messages", [])
        prompt = "\n".join(str(message.get("content", "")) for message in messages)
        result = deterministic_result(prompt)
        self._json({"choices": [{"message": {"content": json.dumps(result)}}]})

    def _json(self, value: object) -> None:
        payload = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, _format: str, *_args: object) -> None:
        return


def deterministic_result(prompt: str) -> dict[str, object]:
    if "executive_summary_markdown" in prompt:
        return {
            "executive_summary_markdown": "# Deterministic completion summary",
            "report_body_markdown": "Verified from command-path evidence.",
        }
    if "problem_recommendation" in prompt:
        return {
            "resolution": "complete",
            "executive_summary": "The recorded evidence meets the criterion.",
            "what_changed": ["Command behavior was verified."],
            "criteria_review": [
                {
                    "criterion": "Verified",
                    "status": "met",
                    "evidence": "Command evidence",
                }
            ],
            "remaining_checklist": [],
            "decision_rationale": "All supplied evidence is complete.",
            "problem_recommendation": "complete",
            "capture_recommendation": "complete",
        }
    if '"markdown":string' in prompt:
        return {"markdown": "# 기존 맥락\n\n재사용 가능한 증거."}
    if '"ko":string,"en":string' in prompt:
        return {"ko": "결정론적 번역", "en": "Deterministic translation"}
    if '"ko":{"summary":string}' in prompt:
        return {
            "ko": {"summary": "결정론적 이미지 요약"},
            "en": {"summary": "Deterministic image summary"},
        }
    if '"validation_criteria":string' in prompt:
        return {
            "ko": {
                "title": "결정론적 솔루션",
                "outcome": "명령 경로가 동작합니다",
                "non_goals": "없음",
                "validation_criteria": "- [ ] 검증됨",
            },
            "en": {
                "title": "Deterministic solution",
                "outcome": "The command path works",
                "non_goals": "None",
                "validation_criteria": "- [ ] Verified",
            },
        }
    if "clear problem statement" in prompt:
        return {
            "ko": {"title": "명확한 문제", "detail": "구조화된 문제 세부 정보"},
            "en": {"title": "Clear problem", "detail": "Structured problem detail"},
        }
    if '"ko":{"title":string,"detail":string}' in prompt:
        return {
            "ko": {"title": "정제된 문제", "detail": "정제된 세부 정보"},
            "en": {"title": "Refined problem", "detail": "Refined detail"},
        }
    if '"title":"refined capture"' in prompt:
        return {"title": "Refined deterministic Capture"}
    if '"entries"' in prompt and "attention_rank" in prompt:
        return {"entries": []}
    if '"claims"' in prompt and "evidence_ids" in prompt:
        return {"claims": []}
    if "Review exactly one Vault passage" in prompt:
        evidence = re.search(r'"evidence"\s*:\s*\{.*?"id"\s*:\s*"([^"]+)"', prompt)
        return {
            "conflict": True,
            "evidence_id": evidence.group(1) if evidence else "",
            "severity": "medium",
            "category": "Command compatibility",
            "summary": "The deterministic evidence requires a human decision.",
            "current_claim": "The command path is covered.",
            "existing_claim": "Reusable evidence must remain authoritative.",
            "impact": "A transport migration could otherwise alter behavior.",
            "recommendation": "Preserve the existing evidence contract.",
            "explanation": "The supplied claims require explicit reconciliation.",
        }
    if "Screen all evidence candidates" in prompt:
        return {"decisions": []}
    if '"conflicts"' in prompt or '"findings"' in prompt:
        return {"conflicts": [], "summary": "No deterministic conflicts."}
    return {}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args()
    ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
