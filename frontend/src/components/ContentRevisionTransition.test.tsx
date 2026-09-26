import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  ContentRevisionTransition,
  type ContentTransitionCause,
} from "./ContentRevisionTransition";

const causes: ContentTransitionCause[] = [
  "automatic_apply",
  "preview_adopted",
  "refinement_apply",
  "version_restore",
];

describe("ContentRevisionTransition", () => {
  it("renders initial and same-revision content statically", () => {
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="capture-1" revision={1} variant="title">
        Initial title
      </ContentRevisionTransition>,
    );

    const root = container.firstElementChild;
    expect(root).toHaveTextContent("Initial title");
    expect(root).not.toHaveAttribute("data-transitioning");

    rerender(
      <ContentRevisionTransition
        entityKey="capture-1"
        revision={1}
        cause="automatic_apply"
        variant="title"
      >
        Localized title
      </ContentRevisionTransition>,
    );
    expect(root).toHaveTextContent("Localized title");
    expect(root).not.toHaveAttribute("data-transitioning");
  });

  it("does not animate a new revision whose rendered content is unchanged", () => {
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="capture-1" revision={1} variant="title">
        Stable title
      </ContentRevisionTransition>,
    );
    rerender(
      <ContentRevisionTransition
        entityKey="capture-1"
        revision={2}
        cause="automatic_apply"
        variant="title"
      >
        Stable title
      </ContentRevisionTransition>,
    );
    expect(container.firstElementChild).not.toHaveAttribute("data-transitioning");
  });

  it.each(causes)("animates an explicit %s revision replacement", (cause) => {
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="capture-1" revision={1} variant="title">
        Rough capture
      </ContentRevisionTransition>,
    );

    rerender(
      <ContentRevisionTransition
        entityKey="capture-1"
        revision={2}
        cause={cause}
        variant="title"
      >
        Organized capture
      </ContentRevisionTransition>,
    );

    const root = container.firstElementChild as HTMLElement;
    expect(root).toHaveAttribute("data-transitioning", "title");
    expect(root.querySelector('[data-content-layer="previous"]')).toHaveTextContent(
      "Rough capture",
    );
    const current = root.querySelector(
      '[data-content-layer="current"]',
    ) as HTMLElement;
    expect(current).toHaveTextContent("Organized capture");
    expect(root.querySelectorAll('[aria-hidden="true"]')).toHaveLength(2);
    expect(root.querySelector('[data-accessible-content="true"]')).toHaveTextContent(
      "Organized capture",
    );

    fireEvent.animationEnd(current);
    expect(root).not.toHaveAttribute("data-transitioning");
    expect(root).toHaveTextContent("Organized capture");
    expect(root.querySelector('[data-content-layer="previous"]')).not.toBeInTheDocument();
  });

  it("uses the quieter body transition contract", () => {
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="capture-1" revision={1} variant="body">
        Original body
      </ContentRevisionTransition>,
    );
    rerender(
      <ContentRevisionTransition
        entityKey="capture-1"
        revision={2}
        cause="automatic_apply"
        variant="body"
      >
        Organized body
      </ContentRevisionTransition>,
    );

    const root = container.firstElementChild as HTMLElement;
    expect(root).toHaveAttribute("data-transitioning", "body");
    expect(root.querySelector('[data-content-layer="previous"]')).not.toBeInTheDocument();
    expect(root.querySelector('[data-content-layer="current"]')).toHaveTextContent(
      "Organized body",
    );
  });

  it("renders formatted content through the same body transition", () => {
    const renderContent = (content: string) => <strong data-formatted="true">{content}</strong>;
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="knowledge-1" revision={1} variant="body" renderContent={renderContent}>
        Original Markdown
      </ContentRevisionTransition>,
    );
    rerender(
      <ContentRevisionTransition entityKey="knowledge-1" revision={2} cause="version_restore" variant="body" renderContent={renderContent}>
        Restored Markdown
      </ContentRevisionTransition>,
    );

    expect(container.querySelector('[data-content-layer="current"] [data-formatted="true"]')).toHaveTextContent("Restored Markdown");
  });

  it("restarts for a rapid newer revision and ignores stale completion events", () => {
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="capture-1" revision={1} variant="title">
        First title
      </ContentRevisionTransition>,
    );
    rerender(
      <ContentRevisionTransition
        entityKey="capture-1"
        revision={2}
        cause="automatic_apply"
        variant="title"
      >
        Second title
      </ContentRevisionTransition>,
    );
    const staleCurrent = container.querySelector(
      '[data-content-layer="current"]',
    ) as HTMLElement;

    rerender(
      <ContentRevisionTransition
        entityKey="capture-1"
        revision={3}
        cause="automatic_apply"
        variant="title"
      >
        Third title
      </ContentRevisionTransition>,
    );

    const root = container.firstElementChild as HTMLElement;
    const latestCurrent = root.querySelector(
      '[data-content-layer="current"]',
    ) as HTMLElement;
    expect(latestCurrent).not.toBe(staleCurrent);
    expect(root.querySelector('[data-content-layer="previous"]')).toHaveTextContent(
      "Second title",
    );
    expect(latestCurrent).toHaveTextContent("Third title");

    fireEvent.animationEnd(staleCurrent);
    expect(root).toHaveAttribute("data-transitioning", "title");
    expect(root.querySelector('[data-content-layer="current"]')).toHaveTextContent(
      "Third title",
    );

    fireEvent(latestCurrent, new Event("animationcancel", { bubbles: true }));
    expect(root).not.toHaveAttribute("data-transitioning");
    expect(root).toHaveTextContent("Third title");
  });

  it("does not animate navigation, historical reading, or reduced motion", () => {
    const { container, rerender } = render(
      <ContentRevisionTransition entityKey="capture-1" revision={1} variant="title">
        First capture
      </ContentRevisionTransition>,
    );
    const root = container.firstElementChild;

    rerender(
      <ContentRevisionTransition
        entityKey="capture-2"
        revision={2}
        cause="automatic_apply"
        variant="title"
      >
        Navigated capture
      </ContentRevisionTransition>,
    );
    expect(root).not.toHaveAttribute("data-transitioning");

    rerender(
      <ContentRevisionTransition
        entityKey="capture-2"
        revision={3}
        cause="version_restore"
        historical
        variant="title"
      >
        Historical capture
      </ContentRevisionTransition>,
    );
    expect(root).not.toHaveAttribute("data-transitioning");

    rerender(
      <ContentRevisionTransition
        entityKey="capture-2"
        revision={4}
        cause="automatic_apply"
        reducedMotion
        variant="title"
      >
        Reduced motion capture
      </ContentRevisionTransition>,
    );
    expect(root).not.toHaveAttribute("data-transitioning");
    expect(root).toHaveTextContent("Reduced motion capture");
  });

  it("keeps the same wrapper focused and in document order while content changes", () => {
    const { container, rerender } = render(
      <div>
        <button type="button">Before</button>
        <ContentRevisionTransition
          as="h3"
          entityKey="capture-1"
          revision={1}
          tabIndex={-1}
          variant="title"
        >
          Rough capture
        </ContentRevisionTransition>
        <button type="button">After</button>
      </div>,
    );
    const root = container.querySelector("h3") as HTMLElement;
    root.focus();

    rerender(
      <div>
        <button type="button">Before</button>
        <ContentRevisionTransition
          as="h3"
          entityKey="capture-1"
          revision={2}
          cause="automatic_apply"
          tabIndex={-1}
          variant="title"
        >
          Organized capture
        </ContentRevisionTransition>
        <button type="button">After</button>
      </div>,
    );

    expect(root).toHaveFocus();
    expect(root.previousElementSibling).toHaveTextContent("Before");
    expect(root.nextElementSibling).toHaveTextContent("After");
  });
});
