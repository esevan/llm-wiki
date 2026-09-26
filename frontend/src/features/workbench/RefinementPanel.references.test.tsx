import { invoke } from "@tauri-apps/api/core";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TauriApplicationClient } from "../../services/tauriApplicationClient";
import type {
  DocumentMention,
  ReferenceWorkspace,
  WorkPreviewFields,
} from "../../types/taskWorkbench";
import { RefinementPanel } from "./RefinementPanel";
import { fixtureVersion, fixtureWorkspace } from "./referencePreviewFixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
type Operation = { name: string; input: Record<string, unknown> };
function nativeFixture(initial?: ReferenceWorkspace) {
  let workspace: ReferenceWorkspace = initial ?? {
    preview: null,
    investigations: [],
    findings: [],
  };
  let taskRevision = 1;
  let draftRevision = 1;
  let responseStatus = "completed";
  let rejectEdit = false;
  let savedText = "";
  let mentionSaveBarrier: Promise<void> | undefined;
  const calls: Operation[] = [];
  const session = () => ({
    id: "session",
    taskId: "task",
    draftRevision,
    responseStatus,
    messages: [],
    inputDraft: "Private note",
  });
  vi.mocked(invoke).mockImplementation(async (_command, args) => {
    const operation = (args as { operation: Operation }).operation;
    calls.push(structuredClone(operation));
    const { name, input } = operation;
    let body: unknown;
    switch (name) {
      case "task-refinement.open":
      case "task-refinement.get":
        body = session();
        break;
      case "task.get":
        body = {
          id: "task",
          taskRevision,
          title: "Contract Task",
          state: "draft",
          detail: "Canonical Task unchanged",
          readiness: { status: "unassessed", blockers: [] },
        };
        break;
      case "task-refinement.proposals":
        body = [
          {
            id: "legacy",
            type: "task_revision",
            payload: { title: "Legacy proposal" },
          },
        ];
        break;
      case "task-refinement.workspace":
        body = {};
        break;
      case "task-refinement.reference-workspace":
        body = structuredClone(workspace);
        break;
      case "task-refinement.reference-generate":
        workspace = {
          ...workspace,
          generation: { id: "job", status: "queued" },
        };
        body = { jobId: "job", status: "queued" };
        break;
      case "task-refinement.reference-investigate":
        workspace = {
          ...workspace,
          investigations: [
            { id: "investigation", state: "running", contextRevision: 1 },
          ],
        };
        body = { status: "running" };
        break;
      case "task-refinement.reference-version":
        body = fixtureVersion(Number(input.version));
        break;
      case "task-refinement.reference-compare":
        body = {
          left: fixtureVersion(Number(input.left)),
          right: fixtureVersion(Number(input.right)),
          comparison: { fields: [{ key: "goal", state: "changed" }] },
        };
        break;
      case "task-refinement.reference-edit":
        if (rejectEdit)
          return { status: 409, body: { error: "preview_head_conflict" } };
        workspace = fixtureWorkspace(Number(input.version) + 1);
        workspace.preview!.current!.fields = input.fields as WorkPreviewFields;
        body = { version: workspace.preview!.currentVersion };
        break;
      case "task-refinement.reference-apply":
        taskRevision += 1;
        body = {
          task: { id: "task", taskRevision },
          transitionCause: "preview_adopted",
          version: input.version,
        };
        break;
      case "task-refinement.reference-restore":
        workspace = fixtureWorkspace(workspace.preview!.currentVersion + 1);
        body = {};
        break;
      case "task-refinement.reference-open":
        body = {
          reference: input,
          source: { markdown: `# ${input.section}\nExact policy body` },
        };
        break;
      case "task-refinement.reference-list":
        body = { items: fixtureVersion().references };
        break;
      case "task-refinement.mention-draft":
        savedText = String(input.text);
        workspace = {
          ...workspace,
          mentionDraft: {
            text: savedText,
            mentions: input.mentions as DocumentMention[],
          },
        };
        if (mentionSaveBarrier) await mentionSaveBarrier;
        body = {};
        break;
      case "task-refinement.message":
        draftRevision += 1;
        responseStatus = "running";
        body = session();
        break;
      default:
        throw new Error(`Unexpected native operation: ${name}`);
    }
    return { status: 200, body };
  });
  window.llmWikiApplication = new TauriApplicationClient();
  return {
    calls,
    pauseMentionSave() {
      let release = () => {};
      mentionSaveBarrier = new Promise<void>((resolve) => {
        release = resolve;
      });
      return () => {
        mentionSaveBarrier = undefined;
        release();
      };
    },
    setWorkspace(value: ReferenceWorkspace) {
      workspace = value;
    },
    rejectEdit() {
      rejectEdit = true;
    },
    get savedText() {
      return savedText;
    },
  };
}
const named = (calls: Operation[], name: string) =>
  calls.filter((call) => call.name === `task-refinement.${name}`);

describe("Mounted reference Refinement through native commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    document.documentElement.lang = "en";
  });
  it("automatically starts the first preview, polls completion, edits and explicitly applies with exact heads", async () => {
    const native = nativeFixture();
    const onApplied = vi.fn();
    render(
      <RefinementPanel
        kind="task"
        subjectId="task"
        onClose={vi.fn()}
        onApplied={onApplied}
      />,
    );
    await waitFor(() =>
      expect(named(native.calls, "reference-generate")).toHaveLength(1),
    );
    expect(named(native.calls, "reference-generate")[0].input).toMatchObject({
      sessionId: "session",
      expectedTaskRevision: 1,
      expectedContextRevision: "refinement:1",
    });
    expect(named(native.calls, "message")).toHaveLength(0);
    native.setWorkspace(fixtureWorkspace());
    await screen.findByText("Contract preview 1", {}, { timeout: 2_000 });
    expect(screen.queryByText("Legacy proposal")).not.toBeInTheDocument();
    expect(named(native.calls, "reference-apply")).toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: "Edit preview" }));
    fireEvent.change(screen.getByLabelText("Edit preview: Goal"), {
      target: { value: "User reviewed goal" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save preview" }));
    await screen.findByText("User reviewed goal");
    expect(named(native.calls, "reference-edit")[0].input).toMatchObject({
      version: 1,
      expectedCurrentPreviewVersion: 1,
      expectedContentHash: "hash-1",
      fields: { goal: "User reviewed goal" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Apply to Task" }));
    await waitFor(() => expect(onApplied).toHaveBeenCalledOnce());
    expect(onApplied).toHaveBeenCalledWith({
      taskId: "task",
      revision: 2,
      cause: "preview_adopted",
    });
    expect(named(native.calls, "reference-apply")[0].input).toMatchObject({
      version: 2,
      expectedCurrentPreviewVersion: 2,
      expectedContentHash: "hash-2",
      expectedTaskRevision: 1,
    });
    expect(named(native.calls, "reference-generate")).toHaveLength(1);
  });
  it("keeps user edits during background polling and preserves the original edit CAS on conflict", async () => {
    const native = nativeFixture(fixtureWorkspace());
    const close = vi.fn();
    render(<RefinementPanel kind="task" subjectId="task" onClose={close} />);
    await screen.findByText("Contract preview 1");
    fireEvent.click(screen.getByRole("button", { name: "Edit preview" }));
    fireEvent.change(screen.getByLabelText("Edit preview: Goal"), {
      target: { value: "Do not overwrite" },
    });
    native.setWorkspace(fixtureWorkspace(2));
    await screen.findByText(
      "A newer preview is available. Your edits are preserved.",
      {},
      { timeout: 2_000 },
    );
    native.rejectEdit();
    fireEvent.click(screen.getByRole("button", { name: "Save preview" }));
    await screen.findByText(
      "The action could not be completed. Your edits are preserved.",
    );
    expect(screen.getByLabelText("Edit preview: Goal")).toHaveValue(
      "Do not overwrite",
    );
    expect(
      named(native.calls, "reference-edit")[0].input
        .expectedCurrentPreviewVersion,
    ).toBe(1);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(close).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel edit" }));
    await screen.findByText("Contract preview 2");
  });
  it("fetches exact saved versions, compares them, restores and opens the selected source in a nested modal", async () => {
    const native = nativeFixture(fixtureWorkspace(2));
    render(<RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />);
    await screen.findByText("Contract preview 2");
    fireEvent.change(screen.getByLabelText("Version"), {
      target: { value: "1" },
    });
    await screen.findByText("Contract preview 1");
    expect(named(native.calls, "reference-version")[0].input.version).toBe(1);
    expect(
      screen.queryByRole("button", { name: "Apply to Task" }),
    ).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Later version"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Compare versions" }));
    await screen.findByRole("region", { name: "Version comparison" });
    expect(named(native.calls, "reference-compare")[0].input).toMatchObject({
      left: 1,
      right: 2,
    });
    const opener = screen.getByRole("button", {
      name: "Approval policy Approval",
    });
    opener.focus();
    fireEvent.click(opener);
    await screen.findByText("Exact policy body");
    expect(named(native.calls, "reference-open")[0].input).toMatchObject({
      version: 1,
      documentId: "approval",
      documentVersion: "source-v1",
      section: "Approval",
    });
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() => expect(opener).toHaveFocus());
    expect(
      screen.getByRole("dialog", { name: "Refining" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Restore as new" }));
    await screen.findAllByText("Contract preview 3");
    expect(named(native.calls, "reference-restore")[0].input).toMatchObject({
      version: 1,
      expectedCurrentPreviewVersion: 2,
    });
  });
  it("persists keyboard mentions without sending, restores them, and allows conversation during investigation", async () => {
    const native = nativeFixture(fixtureWorkspace());
    const first = render(
      <RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />,
    );
    await screen.findByText("Contract preview 1");
    const composer = screen.getByLabelText("Refinement message");
    fireEvent.change(composer, { target: { value: "Use @approval" } });
    await screen.findByRole("listbox");
    expect(named(native.calls, "reference-list")[0].input.query).toBe(
      "approval",
    );
    fireEvent.keyDown(composer, { key: "Enter", isComposing: true });
    expect(composer).toHaveValue("Use @approval");
    fireEvent.keyDown(composer, { key: "ArrowDown" });
    fireEvent.keyDown(composer, { key: "ArrowUp" });
    fireEvent.keyDown(composer, { key: "Enter" });
    expect(composer).toHaveValue("Use @Approval policy > Approval ");
    await waitFor(() =>
      expect(native.savedText).toBe("Use @Approval policy > Approval "),
    );
    expect(named(native.calls, "message")).toHaveLength(0);
    expect(named(native.calls, "mention-draft")[0].input.mentions).toEqual([
      expect.objectContaining({
        documentId: "approval",
        documentVersion: "source-v1",
        section: "Approval",
      }),
    ]);
    first.unmount();
    render(<RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />);
    await waitFor(() =>
      expect(screen.getByLabelText("Refinement message")).toHaveValue(
        "Use @Approval policy > Approval ",
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "Check references" }));
    await screen.findByText("Checking references in the background…");
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect(named(native.calls, "message")).toHaveLength(1));
    expect(named(native.calls, "message")[0].input).toMatchObject({
      message: "Use @Approval policy > Approval",
      mentions: [expect.objectContaining({ documentId: "approval" })],
    });
  });
  it("saves newer composer changes after an older mention save finishes", async () => {
    const native = nativeFixture(fixtureWorkspace());
    const release = native.pauseMentionSave();
    render(<RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />);
    await screen.findByText("Contract preview 1");
    const composer = screen.getByLabelText("Refinement message");
    fireEvent.change(composer, { target: { value: "First unsent note" } });
    await waitFor(() =>
      expect(named(native.calls, "mention-draft")).toHaveLength(1),
    );
    fireEvent.change(composer, { target: { value: "Latest unsent note" } });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 550));
      release();
    });
    await waitFor(() => expect(native.savedText).toBe("Latest unsent note"));
    expect(named(native.calls, "mention-draft")).toHaveLength(2);
    expect(named(native.calls, "message")).toHaveLength(0);
  });
  it("switches reference copy to Korean without losing an unsent mention", async () => {
    nativeFixture(fixtureWorkspace());
    render(<RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />);
    await screen.findByText("Contract preview 1");
    fireEvent.change(screen.getByLabelText("Refinement message"), {
      target: { value: "Still writing" },
    });
    await act(async () => {
      document.documentElement.lang = "ko";
    });
    expect(
      screen.getByRole("button", { name: "초안 편집" }),
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("region", { name: "작업 초안" })).getByLabelText(
        "참조 검색",
      ),
    ).toBeInTheDocument();
    expect(screen.getByDisplayValue("Still writing")).toBeInTheDocument();
    await act(async () => {
      document.documentElement.lang = "en";
    });
  });
  it("inserts an ordered batch without sending and deletes a bound token atomically", async () => {
    const native = nativeFixture(fixtureWorkspace());
    render(<RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />);
    await screen.findByText("Contract preview 1");
    const composer = screen.getByLabelText(
      "Refinement message",
    ) as HTMLTextAreaElement;
    fireEvent.change(composer, { target: { value: "Compare @policy" } });
    const lookup = await screen.findByRole("listbox");
    const options = within(lookup).getAllByRole("option");
    fireEvent.click(options[1]);
    fireEvent.click(options[0]);
    fireEvent.click(
      screen.getByRole("button", { name: "Insert selected mentions" }),
    );
    await waitFor(() =>
      expect(native.savedText).toBe(
        "Compare @Rollout policy > Rollout @Approval policy > Approval ",
      ),
    );
    const saves = named(native.calls, "mention-draft");
    expect(
      (saves.at(-1)!.input.mentions as DocumentMention[]).map(
        (item) => item.documentId,
      ),
    ).toEqual(["rollout", "approval"]);
    expect(named(native.calls, "message")).toHaveLength(0);
    const firstEnd = "Compare @Rollout policy > Rollout".length;
    composer.setSelectionRange(firstEnd, firstEnd);
    fireEvent.keyDown(composer, { key: "Backspace" });
    expect(composer).toHaveValue("Compare  @Approval policy > Approval ");
    await waitFor(() =>
      expect(
        (
          named(native.calls, "mention-draft").at(-1)!.input
            .mentions as DocumentMention[]
        ).map((item) => item.documentId),
      ).toEqual(["approval"]),
    );
    composer.setSelectionRange(9, 9);
    fireEvent.keyDown(composer, { key: "Delete" });
    expect(composer).toHaveValue("Compare   ");
    expect(named(native.calls, "message")).toHaveLength(0);
  });
  it("inserts the active exact mention with Space and leaves pasted labels unbound", async () => {
    const native = nativeFixture(fixtureWorkspace());
    render(<RefinementPanel kind="task" subjectId="task" onClose={vi.fn()} />);
    await screen.findByText("Contract preview 1");
    const composer = screen.getByLabelText("Refinement message");
    fireEvent.change(composer, { target: { value: "Use @approval" } });
    await screen.findByRole("listbox");
    fireEvent.keyDown(composer, { key: " " });
    expect(composer).toHaveValue("Use @Approval policy > Approval ");
    fireEvent.change(composer, {
      target: {
        value:
          "Use @Approval policy > Approval and @Approval policy > Approval",
      },
    });
    await waitFor(() => expect(native.savedText).toContain("and @Approval"));
    expect(
      named(native.calls, "mention-draft").at(-1)!.input.mentions,
    ).toHaveLength(1);
    expect(named(native.calls, "message")).toHaveLength(0);
  });
});
