import { memo, useEffect, useRef, useState } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

const markdownPlugins = [remarkGfm];

export const SessionMarkdown = memo(function SessionMarkdown({ children }: { children: string }) {
  return <div className="task-session-markdown" data-user-content><Markdown remarkPlugins={markdownPlugins} skipHtml>{children}</Markdown></div>;
});

// Keep the same answer container across the stream/final boundary. Completed
// history mounts without motion; only an answer observed streaming can fade in.
export const SessionAnswer = memo(function SessionAnswer({ text, streaming, successful }: { text: string; streaming: boolean; successful: boolean }) {
  const root = useRef<HTMLDivElement>(null);
  const [observedStream, setObservedStream] = useState(streaming);
  const [heldText, setHeldText] = useState<string>();
  useEffect(() => { if (streaming) setObservedStream(true); }, [streaming]);
  useEffect(() => {
    const selectionChanged = () => {
      if (!streaming && heldText === undefined) return;
      const selection = window.getSelection();
      const inside = selection && !selection.isCollapsed && (root.current?.contains(selection.anchorNode) || root.current?.contains(selection.focusNode));
      setHeldText(current => inside ? current ?? root.current?.textContent ?? "" : undefined);
    };
    document.addEventListener("selectionchange", selectionChanged);
    return () => document.removeEventListener("selectionchange", selectionChanged);
  }, [streaming, heldText]);
  const final = !streaming && heldText === undefined;
  return <div ref={root} className={`task-session-answer${final && successful && observedStream ? " task-session-answer-settled" : ""}`} data-answer-state={final ? "final" : "streaming"}>
    {final ? <SessionMarkdown>{text}</SessionMarkdown> : <div className="task-session-stream-text" data-user-content>{heldText ?? text}</div>}
  </div>;
});
