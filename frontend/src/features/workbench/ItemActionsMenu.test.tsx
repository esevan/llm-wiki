import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ItemActionsMenu } from "./ItemActionsMenu";

describe("Item actions menu", () => {
  it.each([['in_progress', ['Put on hold','Move to execution backlog','Delete']], ['on_hold',['Resume','Move to execution backlog','Delete']], ['task',['Delete']], ['completed',['Delete']]] as const)("shows only applicable %s actions in the old delete position", (state, labels) => {
    render(<ItemActionsMenu title="Example" state={state} onMove={() => {}} onDelete={() => {}} />);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', {name:'More actions: Example'}));
    expect(screen.getAllByRole('menuitem').map(item => item.textContent)).toEqual([...labels]);
    expect(screen.getAllByRole('menuitem').at(-1)).toHaveClass('item-actions-delete');
  });
  it("supports keyboard, Escape focus return, Tab and outside dismissal", () => {
    render(<ItemActionsMenu title="Example" state="in_progress" onMove={() => {}} onDelete={() => {}} />);
    const trigger=screen.getByRole('button'); trigger.focus();
    fireEvent.keyDown(trigger,{key:'ArrowDown'});
    expect(screen.getByRole('menuitem',{name:'Put on hold'})).toHaveFocus();
    fireEvent.keyDown(document.activeElement!,{key:'End'}); expect(screen.getByRole('menuitem',{name:'Delete'})).toHaveFocus();
    fireEvent.keyDown(document.activeElement!,{key:'ArrowDown'}); expect(screen.getByRole('menuitem',{name:'Put on hold'})).toHaveFocus();
    fireEvent.keyDown(document.activeElement!,{key:'Escape'}); expect(trigger).toHaveFocus(); expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    fireEvent.click(trigger); fireEvent.keyDown(document.activeElement!,{key:'Tab'}); expect(trigger).toHaveFocus();
    fireEvent.click(trigger); fireEvent.pointerDown(document.body); expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
  it("disables moving active executions but leaves confirmed deletion available", () => {
    const move=vi.fn(); render(<ItemActionsMenu title="Running" state="in_progress" activeExecution onMove={move} onDelete={()=>{}} />);
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByRole('menuitem',{name:'Put on hold'})).toBeDisabled();
    expect(screen.getByRole('menuitem',{name:'Move to execution backlog'})).toBeDisabled();
    expect(screen.getByRole('menuitem',{name:'Delete'})).toHaveFocus();
    expect(screen.getByRole('status')).toHaveTextContent('Finish or stop'); expect(move).not.toHaveBeenCalled();
  });
  it("ignores queued opener scroll events and dismisses on a later scroll", () => {
    render(<div data-testid="scroller"><ItemActionsMenu title="Example" state="in_progress" onMove={() => {}} onDelete={() => {}} /></div>);
    const scroller = screen.getByTestId('scroller');
    scroller.scrollTop = 120;
    fireEvent.click(screen.getByRole('button'));
    fireEvent.scroll(scroller);
    expect(screen.getByRole('menu')).toBeInTheDocument();
    scroller.scrollTop = 160;
    fireEvent.scroll(scroller);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
  it("selects one action, closes before mutation and returns focus", () => {
    const move=vi.fn(), remove=vi.fn(); render(<ItemActionsMenu title="Held" state="on_hold" onMove={move} onDelete={remove} />);
    const trigger=screen.getByRole('button'); fireEvent.click(trigger);
    fireEvent.click(screen.getByRole('menuitem',{name:'Resume'}));
    expect(move).toHaveBeenCalledExactlyOnceWith('resume'); expect(remove).not.toHaveBeenCalled(); expect(trigger).toHaveFocus(); expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
});
