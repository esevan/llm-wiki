import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TaskDetail } from "./TaskDetail";

const task = {
  id: "task-1",
  kind: "task" as const,
  taskRevision: 2,
  state: "in_progress" as const,
  title: "Independent work",
  workLog: [],
  checklist: [],
  decisions: [],
  readinessEntries: [
    { key: "scope", status: "missing" as const },
    { key: "validationCriteria", status: "resolved" as const },
  ],
};
const response = (body: unknown) => ({
  ok: true,
  status: 200,
  json: async () => body,
  text: async () => "",
  body: null,
});

describe("Task detail", () => {
  it("keeps dependent mutations inert until the current revision is rendered", async () => {
    let finishWorkLog!: (value: ReturnType<typeof response>) => void;
    const workLog = new Promise<ReturnType<typeof response>>((resolve) => {
      finishWorkLog = resolve;
    });
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) =>
      path === "/tasks/task-1/work-log" && method === "POST"
        ? workLog
        : Promise.resolve(response(task)),
    );
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    const workLogField = await screen.findByLabelText("Work Log entry");
    fireEvent.change(workLogField, { target: { value: "Revision-bound evidence" } });
    fireEvent.click(workLogField.parentElement!.querySelector("button")!);
    const detail = screen.getByLabelText("Independent work");
    await waitFor(() => expect(detail).toHaveAttribute("aria-busy", "true"));

    const decisionField = screen.getByLabelText("Decision");
    fireEvent.change(decisionField, { target: { value: "Must wait" } });
    fireEvent.click(decisionField.parentElement!.querySelector("button")!);
    expect(request.mock.calls.some(([input]) => input.path === "/tasks/task-1/decisions")).toBe(false);

    finishWorkLog(response(task));
    await waitFor(() => expect(detail).toHaveAttribute("aria-busy", "false"));
  });

  it("requires authored completion evidence and permits work before readiness is resolved", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    await screen.findByText("Independent work");
    expect(screen.getAllByText("Scope")).toHaveLength(2);
    expect(screen.getAllByText("Validation criteria")).toHaveLength(2);
    expect(screen.getByText("Missing")).toBeVisible();
    expect(screen.queryByText("validationCriteria")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Complete Task" }),
    ).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Completion evidence"), {
      target: { value: "Validated in a real workflow" },
    });
    expect(screen.getByRole("button", { name: "Complete Task" })).toBeEnabled();
    fireEvent.change(screen.getByLabelText("Work Log entry"), {
      target: { value: "Started gathering evidence" },
    });
    fireEvent.click(screen.getAllByRole("button", { name: "Add" })[0]);
    await waitFor(() =>
      expect(request).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "/tasks/task-1/work-log",
          method: "POST",
        }),
      ),
    );
  });

  it("sends the current Task revision and the checkbox event value", async () => {
    const checklistTask = {
      ...task,
      checklist: [{ id: "check-1", body: "Verify result", checked: false }],
    };
    const request = vi.fn().mockResolvedValue(response(checklistTask));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );

    fireEvent.click(await screen.findByRole("checkbox", { name: "Verify result" }));

    await waitFor(() => {
      const checklistUpdate = request.mock.calls
        .map(([input]) => input)
        .find(
          (input) =>
            input.path === "/tasks/task-1/checklist/check-1" &&
            input.method === "PUT",
        );
      expect(JSON.parse(checklistUpdate.body)).toMatchObject({
        expectedTaskRevision: 2,
        checked: true,
        body: "Verify result",
      });
    });
  });

  it("maps an authored decision to the append-only decision contract", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    const decisionField = await screen.findByLabelText("Decision");
    fireEvent.change(decisionField, {
      target: { value: "Ship deliberately" },
    });
    fireEvent.click(decisionField.parentElement!.querySelector("button")!);

    await waitFor(() => {
      const decision = request.mock.calls
        .map(([input]) => input)
        .find((input) => input.path === "/tasks/task-1/decisions");
      expect(JSON.parse(decision.body)).toMatchObject({
        expectedTaskRevision: 2,
        kind: "user",
        payload: { body: "Ship deliberately" },
      });
    });
  });
});
