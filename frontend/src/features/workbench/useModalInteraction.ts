import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

type Modal = { root: RefObject<HTMLElement | null>; priority: number };
const modals: Modal[] = [];
let background: { element: HTMLElement; inert: boolean } | undefined;

function topModal() {
  // Refinement can mount alongside or inside Task detail. React effect order
  // does not necessarily match their visual stacking order.
  return modals.reduce<Modal | undefined>((top, modal) =>
    modal.root.current?.isConnected && (!top || modal.priority >= top.priority) ? modal : top, undefined);
}

function nativeModalOpen() {
  try { return Boolean(document.querySelector("dialog:modal")); }
  catch { return Boolean(document.querySelector("dialog[open]")); }
}

function available(element: HTMLElement) {
  if (!element.isConnected || element.matches(":disabled, input[type='hidden']") || element.closest("[hidden], [inert]")) return false;
  for (let ancestor: HTMLElement | null = element; ancestor; ancestor = ancestor.parentElement) {
    const style = getComputedStyle(ancestor);
    if (style.display === "none" || style.visibility === "hidden") return false;
    if (ancestor instanceof HTMLDetailsElement && !ancestor.open
      && !ancestor.querySelector("summary")?.contains(element)) return false;
  }
  return true;
}

function tabStops(root: HTMLElement) {
  return Array.from(root.querySelectorAll<HTMLElement>(
    "button, textarea, input, select, summary, a[href], [tabindex]",
  )).filter(element => element.tabIndex >= 0 && available(element));
}

/** Own Escape and focus even when a disabled/removed control loses focus. */
export function useModalInteraction(root: RefObject<HTMLElement | null>, priority: number, onEscape: () => void) {
  const escape = useRef(onEscape);
  useLayoutEffect(() => { escape.current = onEscape; }, [onEscape]);
  useEffect(() => {
    const modal = { root, priority };
    if (!modals.length) {
      const element = document.querySelector<HTMLElement>(".app");
      if (element) {
        background = { element, inert: element.inert };
        element.inert = true;
      }
    }
    modals.push(modal);
    let lastFocused: HTMLElement | undefined;
    const ownsInput = () => topModal() === modal && !nativeModalOpen();
    const repairFocus = () => {
      const panel = root.current;
      if (!panel || !ownsInput()) return;
      const active = document.activeElement;
      if (active instanceof HTMLElement && panel.contains(active) && available(active)) {
        lastFocused = active;
        return;
      }
      const target = lastFocused && panel.contains(lastFocused) && available(lastFocused)
        ? lastFocused : tabStops(panel)[0] ?? panel;
      target.focus({ preventScroll: true });
    };
    const keydown = (event: KeyboardEvent) => {
      if (!ownsInput() || event.defaultPrevented || event.isComposing || event.keyCode === 229) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        if (!event.repeat) escape.current();
      } else if (event.key === "Tab" && root.current) {
        const stops = tabStops(root.current);
        const index = stops.indexOf(document.activeElement as HTMLElement);
        if (index < 0 || (event.shiftKey ? index === 0 : index === stops.length - 1)) {
          event.preventDefault();
          event.stopPropagation();
          const target = (event.shiftKey ? stops[stops.length - 1] : stops[0]) ?? root.current;
          target.focus({ preventScroll: true });
        }
      }
    };
    document.addEventListener("keydown", keydown, true);
    document.addEventListener("focusin", repairFocus, true);
    window.addEventListener("focus", repairFocus);
    // Disabling/removing a focused control need not emit a focusin event.
    const observer = new MutationObserver(repairFocus);
    observer.observe(document.body, { subtree: true, childList: true, attributes: true,
      attributeFilter: ["disabled", "hidden", "inert", "open"] });
    const initialFocus = requestAnimationFrame(repairFocus);
    return () => {
      cancelAnimationFrame(initialFocus);
      observer.disconnect();
      document.removeEventListener("keydown", keydown, true);
      document.removeEventListener("focusin", repairFocus, true);
      window.removeEventListener("focus", repairFocus);
      modals.splice(modals.indexOf(modal), 1);
      if (!modals.length && background) {
        background.element.inert = background.inert;
        background = undefined;
      }
    };
  }, [root, priority]);
}
