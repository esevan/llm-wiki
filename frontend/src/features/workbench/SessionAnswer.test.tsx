import { act, fireEvent, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import sessionStyles from "./task-sessions.css?raw";
import { SessionAnswer, SessionMarkdown } from "./SessionAnswer";
const parsed = vi.hoisted(() => vi.fn());
vi.mock("react-markdown", () => ({ default: ({ children }: { children: string }) => { parsed(children); return <p>{children}</p>; } }));

describe("session answer rendering", () => {
  it("keeps historical Markdown stable and parses the completed authoritative answer once", () => {
    parsed.mockClear();
    const view = (text: string, streaming: boolean) => <><SessionMarkdown>Historical **answer**</SessionMarkdown><SessionAnswer text={text} streaming={streaming} successful={!streaming} /></>;
    const { rerender, container, unmount } = render(view("First", true));
    for (let i=0;i<100;i++) rerender(view(`First ${i}`, true));
    expect(parsed.mock.calls).toEqual([["Historical **answer**"]]);
    const answer = container.querySelector(".task-session-answer");
    rerender(view("Full **authoritative** answer with final tail", false));
    expect(container.querySelector(".task-session-answer")).toBe(answer);
    expect(answer).toHaveClass("task-session-answer-settled");
    expect(parsed.mock.calls).toEqual([["Historical **answer**"], ["Full **authoritative** answer with final tail"]]);
    for (let i=0;i<10;i++) rerender(view("Full **authoritative** answer with final tail", false));
    expect(parsed).toHaveBeenCalledTimes(2);
    unmount();
    const history = render(<SessionAnswer text="Full **authoritative** answer with final tail" streaming={false} successful />);
    expect(history.container.querySelector(".task-session-answer")).not.toHaveClass("task-session-answer-settled");
  });

  it("keeps selected streaming text and defers final parsing until selection is released", () => {
    const { container, rerender } = render(<SessionAnswer text="Selected stream" streaming successful={false} />);
    const node=container.querySelector(".task-session-stream-text")!.firstChild!;
    const selection=window.getSelection()!;const range=document.createRange();range.setStart(node,0);range.setEnd(node,8);selection.removeAllRanges();selection.addRange(range);
    fireEvent(document,new Event("selectionchange"));
    rerender(<SessionAnswer text="Selected stream and authoritative tail" streaming={false} successful />);
    expect(selection.toString()).toBe("Selected");
    expect(container.querySelector('[data-answer-state="streaming"]')).toBeInTheDocument();
    act(()=>selection.removeAllRanges());fireEvent(document,new Event("selectionchange"));
    expect(container.querySelector('[data-answer-state="final"]')).toHaveTextContent("authoritative tail");
  });

  it("does not replace or reparse settled Markdown when selecting historical text", () => {
    parsed.mockClear();
    const {container}=render(<SessionAnswer text="Historical prose" streaming={false} successful />);
    const node=container.querySelector("p")!.firstChild!;
    const range=document.createRange();range.setStart(node,0);range.setEnd(node,10);
    const selection=window.getSelection()!;selection.removeAllRanges();selection.addRange(range);
    fireEvent(document,new Event("selectionchange"));
    expect(selection.toString()).toBe("Historical");
    expect(container.querySelector("p")!.firstChild).toBe(node);
    act(()=>selection.removeAllRanges());fireEvent(document,new Event("selectionchange"));
    expect(parsed).toHaveBeenCalledTimes(1);
  });

  it("disables the final fade in the system reduced-motion stylesheet branch", () => {
    const style=document.createElement("style");style.textContent=sessionStyles;document.head.append(style);
    const reduced=Array.from(style.sheet!.cssRules).filter(rule=>rule.type===CSSRule.MEDIA_RULE).map(rule=>rule as CSSMediaRule).filter(rule=>rule.conditionText.replace(/\s/g,"")==="(prefers-reduced-motion:reduce)");
    const answerRule=reduced.flatMap(rule=>Array.from(rule.cssRules)).map(rule=>rule as CSSStyleRule).find(rule=>rule.selectorText===".task-session-answer-settled");
    expect(answerRule?.style.getPropertyValue("animation")).toBe("none");style.remove();
  });

  it("settles failure tails without a success animation", () => {
    const {rerender,container}=render(<SessionAnswer text="Partial" streaming successful={false}/>);
    rerender(<SessionAnswer text="Partial final fragment" streaming={false} successful={false}/>);
    expect(container.querySelector('[data-answer-state="final"]')).toHaveTextContent("final fragment");
    expect(container.querySelector(".task-session-answer-settled")).toBeNull();
  });
});
