import { useEffect, useRef } from "react";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export function DeleteItemDialog({ title, busy, error, onConfirm, onCancel }: {
  title: string;
  busy: boolean;
  error: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const text = useTaskWorkbenchText();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    const dialog = dialogRef.current!;
    dialog.showModal();
    cancelRef.current?.focus();
    return () => {
      dialog.close();
      const target = opener?.isConnected ? opener : document.querySelector<HTMLElement>("#workbench.active h1");
      target?.focus();
    };
  }, []);
  return (
    <dialog ref={dialogRef} className="workbench-delete-dialog" role="alertdialog"
      aria-labelledby="workbench-delete-title" aria-describedby="workbench-delete-explanation"
      aria-busy={busy} onCancel={(event) => { event.preventDefault(); if (!busy) onCancel(); }}>
      <h2 id="workbench-delete-title">{text.deleteConfirm}</h2>
      <p className="workbench-delete-subject"><strong>{title}</strong></p>
      <p id="workbench-delete-explanation">{text.deleteExplanation}</p>
      {error && <p role="alert">{error}</p>}
      <footer>
        <button ref={cancelRef} type="button" data-control="task-delete-cancel" disabled={busy} onClick={onCancel}>{text.deleteCancel}</button>
        <button type="button" className="workbench-delete-confirm" data-control="task-delete-confirm" disabled={busy} onClick={onConfirm}>{busy ? "…" : text.delete}</button>
      </footer>
    </dialog>
  );
}
