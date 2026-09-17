import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { OverlayLayer } from "../overlays/OverlayLayer";
import { WorkbenchView } from "../workbench/WorkbenchView";
import { WorkTrackingCards } from "./WorkTrackingCards";

const acceptance = describe;

acceptance("release acceptance: real user surfaces", () => {
  it("US1 and US4 expose a live in-app milestone card surface", () => {
    render(<OverlayLayer />);
    // The real runtime publishes a server-issued preview after Track this chat.
    // An idle dialog must not fabricate an accepted or pending Capture.
    const action = vi.fn();
    window.addEventListener('llm-wiki:chat-tracking-action', action);
    act(() => window.dispatchEvent(new CustomEvent('llm-wiki:chat-tracking', {detail:{cards:[{id:'server-review',stage:'capture',title:'Preview',summary:'Needs consent',revision:0}]}})));
    const region = screen.getByRole("region", {
      name: "Tracked work",
      hidden: true,
    });
    expect(
      within(region).getByRole("article", { hidden: true }),
    ).toBeInTheDocument();
    fireEvent.click(within(region).getByRole('button', {name:'Accept',hidden:true}));
    expect(action).toHaveBeenCalledOnce();
    expect((action.mock.calls[0][0] as CustomEvent).detail).toMatchObject({id:'server-review',action:'accept'});
    window.removeEventListener('llm-wiki:chat-tracking-action', action);
  });

  it("US5 renders refreshed tracked state in Workbench", async () => {
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => "",
        json: async () => ({
          workspaceRevision: 9,
          activeSelection: { title: "Tracked from Chat" },
          activeWork: [
            { capture: "Tracked from Chat", state: "active", headRevision: 9 },
          ],
        }),
        body: null,
      }),
    };
    render(<WorkbenchView active />);
    await waitFor(() =>
      expect(window.llmWikiApplication.request).toHaveBeenCalled(),
    );
    expect(await screen.findByText("Tracked from Chat")).toBeVisible();
  });

  it("US3 requires an explicit draft review before publication", () => {
    const publish = vi.fn();
    render(
      <WorkTrackingCards
        cards={[
          {
            id: "offer",
            stage: "knowledge",
            title: "Completed work",
            summary: "Still private",
            revision: 4,
            publicationState: "offered",
          },
        ]}
        onPublish={publish}
      />,
    );
    expect(
      screen.queryByRole("button", { name: "Publish Knowledge" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Review Knowledge draft" }),
    ).toBeVisible();
  });

  it("FR-031 localizes tracked-work actions in Korean", () => {
    document.documentElement.lang = "ko";
    render(
      <WorkTrackingCards
        cards={[
          {
            id: "problem",
            stage: "problem",
            title: "문제 제안",
            summary: "검토가 필요합니다",
            revision: 2,
          },
        ]}
      />,
    );
    expect(screen.getByRole("button", { name: "수락" })).toBeVisible();
    expect(screen.getByRole("button", { name: "수정" })).toBeVisible();
    expect(screen.getByRole("button", { name: "거절" })).toBeVisible();
  });
});
