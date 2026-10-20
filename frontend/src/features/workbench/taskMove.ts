export function taskMoveError(cause: unknown, text: { activeExecutionGuard: string; taskHeldGuard: string }) {
  const message = String(cause instanceof Error ? cause.message : cause);
  return message.includes('active_task_execution') ? text.activeExecutionGuard : message.includes('task_on_hold') ? text.taskHeldGuard : message;
}
