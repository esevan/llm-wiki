import { useRef, type ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useModalInteraction } from "./useModalInteraction";

function Modal({ name, priority = 10, onEscape, children }: {
  name: string; priority?: number; onEscape: () => void; children?: ReactNode;
}) {
  const root = useRef<HTMLDivElement>(null);
  useModalInteraction(root, priority, onEscape);
  return <div ref={root} tabIndex={-1} role="dialog" aria-label={name}>
    <button>{name}</button>{children}
  </div>;
}

function escape(target: HTMLElement | Document = document.body, options: KeyboardEventInit = {}) {
  const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true, ...options });
  fireEvent(target, event);
  return event;
}

describe("modal keyboard and focus ownership", () => {
  it("consumes Escape outside the dialog and releases the listener on unmount", () => {
    const close = vi.fn();
    const view = render(<Modal name="Task" onEscape={close} />);
    expect(escape().defaultPrevented).toBe(true);
    expect(close).toHaveBeenCalledOnce();
    view.unmount();
    expect(escape().defaultPrevented).toBe(false);
    expect(close).toHaveBeenCalledOnce();
  });

  it("uses visual priority even if the lower modal registers later", () => {
    const refine = vi.fn(), task = vi.fn();
    const view = render(<>
      <Modal key="refine" name="Refine" priority={20} onEscape={refine} />
      <Modal key="task" name="Task" onEscape={task} />
    </>);
    escape(screen.getByRole("button", { name: "Task" }));
    expect(refine).toHaveBeenCalledOnce();
    expect(task).not.toHaveBeenCalled();
    view.rerender(<Modal key="task" name="Task" onEscape={task} />);
    expect(escape(document.body, { repeat: true }).defaultPrevented).toBe(true);
    expect(task).not.toHaveBeenCalled();
    escape();
    expect(task).toHaveBeenCalledOnce();
  });

  it("leaves IME composition cancellation alone", () => {
    const close = vi.fn();
    render(<Modal name="Task" onEscape={close} />);
    expect(escape(document.body, { isComposing: true }).defaultPrevented).toBe(false);
    expect(escape(document.body, { keyCode: 229 }).defaultPrevented).toBe(false);
    expect(close).not.toHaveBeenCalled();
  });

  it("recovers escaped focus and focus on a newly disabled control", async () => {
    const close = vi.fn();
    const view = render(<><button>Outside</button><Modal name="Task" onEscape={close}><input aria-label="Message" /></Modal></>);
    const input = screen.getByRole("textbox");
    input.focus();
    screen.getByRole("button", { name: "Outside" }).focus();
    expect(document.activeElement).toBe(input);
    view.rerender(<><button>Outside</button><Modal name="Task" onEscape={close}><input aria-label="Message" disabled /></Modal></>);
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Task" })));
    fireEvent.keyDown(document.body, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Task" }));
  });

  it("consumes Escape when closing is temporarily blocked and uses the latest callback", () => {
    const close = vi.fn();
    const view = render(<Modal name="Task" onEscape={() => {}} />);
    expect(escape().defaultPrevented).toBe(true);
    expect(close).not.toHaveBeenCalled();
    view.rerender(<Modal name="Task" onEscape={close} />);
    escape();
    expect(close).toHaveBeenCalledOnce();
  });

  it("defers to a native modal dialog", () => {
    const close = vi.fn();
    render(<Modal name="Task" onEscape={close} />);
    const native = document.createElement("dialog");
    const spy = vi.spyOn(document, "querySelector").mockReturnValueOnce(native);
    try {
      expect(escape().defaultPrevented).toBe(false);
      expect(close).not.toHaveBeenCalled();
    } finally { spy.mockRestore(); }
  });

  it("keeps the background inert until the last modal closes, regardless of removal order", () => {
    const app = document.createElement("div");
    app.className = "app";
    app.inert = false;
    document.body.append(app);
    const close = vi.fn();
    const view = render(<>
      <Modal key="task" name="Task" onEscape={close} />
      <Modal key="refine" name="Refine" priority={20} onEscape={close} />
    </>);
    expect(app.inert).toBe(true);
    view.rerender(<Modal key="refine" name="Refine" priority={20} onEscape={close} />);
    expect(app.inert).toBe(true);
    view.unmount();
    expect(app.inert).toBe(false);
    app.remove();
  });
});
