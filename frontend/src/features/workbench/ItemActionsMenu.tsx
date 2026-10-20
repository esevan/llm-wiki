import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { TaskState } from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

function focusMenuButton(menu: HTMLDivElement | null, button?: HTMLButtonElement | null) {
  if (!menu || !button) return;
  button.focus({ preventScroll: true });
  const item = button.getBoundingClientRect(), viewport = menu.getBoundingClientRect();
  if (item.top < viewport.top) menu.scrollTop -= viewport.top - item.top;
  else if (item.bottom > viewport.bottom) menu.scrollTop += item.bottom - viewport.bottom;
}

export type TaskMove = "on_hold" | "resume" | "backlog";
export function ItemActionsMenu({ title, state, busy, activeExecution, onMove, onDelete }: {
  title: string; state?: TaskState; busy?: boolean; activeExecution?: boolean;
  onMove?: (to: TaskMove) => void; onDelete?: () => void;
}) {
  const text = useTaskWorkbenchText();
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const trigger = useRef<HTMLButtonElement>(null), menu = useRef<HTMLDivElement>(null);
  const id = useId();
  const close = (restore = true) => { setOpen(false); if (restore) trigger.current?.focus({ preventScroll: true }); };
  useLayoutEffect(() => {
    if (!open) return;
    const box = trigger.current!.getBoundingClientRect();
    const height = menu.current!.getBoundingClientRect().height;
    setPosition({ left: Math.max(8, Math.min(box.right - 248, innerWidth - 256)), top: Math.max(8, Math.min(box.bottom + 4, innerHeight - height - 8)) });
    focusMenuButton(menu.current, menu.current?.querySelector<HTMLButtonElement>('button:not(:disabled)'));
  }, [open]);
  useEffect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => { if (!menu.current?.contains(e.target as Node) && !trigger.current?.contains(e.target as Node)) close(false); };
    // Focusing/clicking an opener can queue a scroll event before the menu
    // mounts. Ignore that already-applied scroll; dismiss on subsequent moves.
    const scrollPosition = (target: EventTarget) => target === document || target === window
      ? `${window.scrollX}:${window.scrollY}` : target instanceof HTMLElement ? `${target.scrollLeft}:${target.scrollTop}` : '';
    const positions = new Map<EventTarget, string>([[document, scrollPosition(document)], [window, scrollPosition(window)]]);
    for (let ancestor: HTMLElement | null = trigger.current; ancestor; ancestor = ancestor.parentElement) positions.set(ancestor, scrollPosition(ancestor));
    const dismiss = (event: Event) => {
      if (event.target instanceof Node && menu.current?.contains(event.target)) return;
      if (event.type === 'scroll' && event.target && positions.get(event.target) === scrollPosition(event.target)) return;
      close(false);
    };
    document.addEventListener('pointerdown', outside);
    window.addEventListener('resize', dismiss);
    window.addEventListener('scroll', dismiss, true);
    return () => { document.removeEventListener('pointerdown', outside); window.removeEventListener('resize', dismiss); window.removeEventListener('scroll', dismiss, true); };
  }, [open]);
  const action = (run: () => void) => { close(); run(); };
  if (!onDelete && !(onMove && (state === "in_progress" || state === "on_hold"))) return null;
  return <>
    <button ref={trigger} type="button" className="item-actions-trigger" data-control="item-actions-menu" aria-label={`${text.moreActions}: ${title}`} aria-haspopup="menu" aria-expanded={open} aria-controls={open ? id : undefined} disabled={busy}
      onClick={() => open ? close() : setOpen(true)} onKeyDown={e => { if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); setOpen(true); } }}>⋯</button>
    {open && createPortal(<div ref={menu} id={id} role="menu" aria-label={`${text.moreActions}: ${title}`} className="item-actions-popover" style={position}
      onKeyDown={e => {
        const buttons = [...menu.current!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(); }
        else if (e.key === 'Tab') { e.preventDefault(); close(); }
        else if (['ArrowDown','ArrowUp','Home','End'].includes(e.key)) { e.preventDefault(); const next = e.key === 'Home' ? 0 : e.key === 'End' ? buttons.length - 1 : (index + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length; focusMenuButton(menu.current, buttons[next]); }
      }}>
      {state === 'in_progress' && onMove && <button type="button" role="menuitem" disabled={activeExecution} data-control="task-menu-hold" onClick={() => action(() => onMove('on_hold'))}>{text.hold}</button>}
      {state === 'on_hold' && onMove && <button type="button" role="menuitem" data-control="task-menu-resume" onClick={() => action(() => onMove('resume'))}>{text.resumeTask}</button>}
      {(state === 'in_progress' || state === 'on_hold') && onMove && <button type="button" role="menuitem" disabled={activeExecution} data-control="task-menu-backlog" onClick={() => action(() => onMove('backlog'))}>{text.executionBacklog}</button>}
      {activeExecution && (state === "in_progress" || state === "on_hold") && <p className="item-actions-hint" role="status">{text.activeExecutionGuard}</p>}
      {onDelete && <button type="button" role="menuitem" className="item-actions-delete" data-control="task-card-delete" onClick={() => action(onDelete)}>{text.delete}</button>}
    </div>, document.body)}
  </>;
}
