import "../features/workbench/reference-aware-workbench.css";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { KnowledgeMarkdown } from "./KnowledgeMarkdown";
import { useModalInteraction } from "../features/workbench/useModalInteraction";
import { useReferenceWorkbenchText } from "../features/workbench/referenceWorkbenchText";
import type { ExactReferenceBinding } from "../types/taskWorkbench";

import { referenceKey } from "./referenceIdentity";

export interface ReferenceDocument extends ExactReferenceBinding {
  markdown: string;
}
export interface ReferenceViewerProps {
  open: boolean;
  binding?: ExactReferenceBinding;
  references: ExactReferenceBinding[];
  read: (binding: ExactReferenceBinding) => Promise<ReferenceDocument>;
  onViewed?: (binding: ExactReferenceBinding) => void;
  onClose: () => void;
  onNavigate: (binding: ExactReferenceBinding) => void;
}
export function ReferenceViewer(props: ReferenceViewerProps) {
  return props.open && props.binding ? (
    <OpenReferenceViewer {...props} binding={props.binding} />
  ) : null;
}
function OpenReferenceViewer({
  binding,
  references,
  read,
  onClose,
  onNavigate,
  onViewed,
}: ReferenceViewerProps & { binding: ExactReferenceBinding }) {
  const text = useReferenceWorkbenchText();
  const panel = useRef<HTMLElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const origin = useRef(
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  );
  const [history, setHistory] = useState([binding]);
  const [index, setIndex] = useState(0);
  const cursor = useRef(0);
  const navigatingHistory = useRef(false);
  const [source, setSource] = useState<ReferenceDocument>();
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(true);
  const [retry, setRetry] = useState(0);
  const callbacks = useRef({ read, onViewed });
  callbacks.current = { read, onViewed };
  const key = referenceKey(binding);
  useModalInteraction(panel, 30, onClose);
  useEffect(() => {
    const opener = origin.current;
    closeButton.current?.focus({ preventScroll: true });
    return () => {
      requestAnimationFrame(() => {
        if (opener?.isConnected) opener.focus({ preventScroll: true });
      });
    };
  }, []);
  useEffect(() => {
    if (navigatingHistory.current) {
      navigatingHistory.current = false;
      return;
    }
    setHistory((previous) => {
      if (referenceKey(previous[cursor.current]) === key) return previous;
      const next = [...previous.slice(0, cursor.current + 1), binding];
      cursor.current = next.length - 1;
      setIndex(cursor.current);
      return next;
    });
  }, [key, binding]);
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(false);
    setSource(undefined);
    void callbacks.current
      .read(binding)
      .then((result) => {
        if (cancelled) return;
        setSource(result);
        callbacks.current.onViewed?.(binding);
      })
      .catch(() => {
        if (!cancelled) setError(true);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [key, binding, retry]);
  useEffect(() => {
    if (!source) return;
    const section = binding.section?.split(" > ").at(-1)?.toLowerCase();
    const heading = Array.from(
      content.current?.querySelectorAll<HTMLElement>("h1,h2,h3,h4,h5,h6") ?? [],
    ).find((node) => node.textContent?.toLowerCase() === section);
    heading?.scrollIntoView?.({ block: "start", behavior: "instant" });
  }, [source, binding.section]);
  const moveHistory = (next: number) => {
    navigatingHistory.current = true;
    cursor.current = next;
    setIndex(next);
    onNavigate(history[next]);
  };
  const selected = references.findIndex(
    (reference) => referenceKey(reference) === key,
  );
  return createPortal(
    <div className="reference-viewer-layer">
      <div
        className="reference-viewer-backdrop"
        onClick={onClose}
        aria-hidden="true"
      />
      <section
        ref={panel}
        className="reference-viewer"
        role="dialog"
        aria-modal="true"
        aria-labelledby="reference-viewer-title"
        tabIndex={-1}
      >
        <header>
          <div>
            <h2 id="reference-viewer-title">
              {binding.title || text.references}
            </h2>
            <small>
              {text.sourceVersion}: {binding.documentVersion}
            </small>
            {binding.section && <p>{binding.section}</p>}
          </div>
          <button
            ref={closeButton}
            data-control="reference-viewer-close"
            type="button"
            onClick={onClose}
            aria-label={text.close}
          >
            ×
          </button>
        </header>
        <nav aria-label={text.navigation}>
          <button
            type="button"
            data-control="reference-viewer-back"
            disabled={index <= 0}
            onClick={() => moveHistory(index - 1)}
          >
            {text.back}
          </button>
          <button
            type="button"
            data-control="reference-viewer-forward"
            disabled={index >= history.length - 1}
            onClick={() => moveHistory(index + 1)}
          >
            {text.forward}
          </button>
          <button
            type="button"
            data-control="reference-viewer-previous"
            disabled={selected <= 0}
            onClick={() => onNavigate(references[selected - 1])}
          >
            {text.previous}
          </button>
          <button
            type="button"
            data-control="reference-viewer-next"
            disabled={selected < 0 || selected >= references.length - 1}
            onClick={() => onNavigate(references[selected + 1])}
          >
            {text.next}
          </button>
        </nav>
        {loading && <p role="status">{text.loadingReference}</p>}
        {error && (
          <p role="alert">
            {text.readFailed}{" "}
            <button
              data-control="reference-viewer-retry"
              type="button"
              onClick={() => setRetry((value) => value + 1)}
            >
              {text.retry}
            </button>
          </p>
        )}
        {source && (
          <div
            ref={content}
            className="reference-viewer-content"
            onClick={(event) => {
              const link = (
                event.target as HTMLElement
              ).closest<HTMLAnchorElement>("a[href]");
              if (!link) return;
              const href = link.getAttribute("href") ?? "";
              if (/^https?:\/\//i.test(href)) {
                link.target = "_blank";
                link.rel = "noopener noreferrer";
                return;
              }
              event.preventDefault();
              const [path, anchor] = href.split("#");
              let decodedAnchor: string | undefined = anchor;
              let linkedPath = path;
              try {
                decodedAnchor = anchor ? decodeURIComponent(anchor) : undefined;
                if (path)
                  linkedPath = decodeURIComponent(
                    new URL(
                      path,
                      `https://vault.local/${binding.path ?? ""}`,
                    ).pathname.slice(1),
                  );
              } catch {
                return;
              }
              const target = references.find(
                (reference) =>
                  (!path
                    ? reference.documentId === binding.documentId
                    : reference.path === linkedPath) &&
                  (!anchor ||
                    reference.section
                      ?.split(" > ")
                      .at(-1)
                      ?.toLowerCase()
                      .replace(/\s+/g, "-") === decodedAnchor?.toLowerCase()),
              );
              if (target) onNavigate(target);
            }}
          >
            <KnowledgeMarkdown>{source.markdown}</KnowledgeMarkdown>
          </div>
        )}
      </section>
    </div>,
    document.body,
  );
}
