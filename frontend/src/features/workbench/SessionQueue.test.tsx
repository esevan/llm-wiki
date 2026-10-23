import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {expect,it,vi} from "vitest";
import {SessionQueue} from "./SessionQueue";
import type {TaskQueue,TaskExecutionRun} from "../../types/taskWorkbench";
const queue:TaskQueue={control:{mode:"active",reason:"",revision:1},items:[{id:"one",instruction:"First",revision:2,position:1,settings:{model:"sol",cwd:"/project",resolvedEffort:"high",approvalsReviewer:"user"}},{id:"two",instruction:"Second",revision:1,position:2,settings:{model:"luna",cwd:"/project",resolvedEffort:"low",approvalsReviewer:"user"}}]};
const run={id:"run",status:"running",stopRequested:false} as TaskExecutionRun;
it("edits and reorders exact pending revisions; stop-and-drain is a separate explicit action",async()=>{
 const action=vi.fn().mockResolvedValue(undefined);render(<SessionQueue queue={queue} currentRun={run} readOnly={false} busy={false} taskId="task" sessionId="session" onAction={action}/>);
 fireEvent.click(screen.getAllByRole("button",{name:"Edit"})[0]);fireEvent.change(screen.getByLabelText("Edit pending request"),{target:{value:"Edited"}});fireEvent.click(screen.getByRole("button",{name:"Save changes"}));
 await waitFor(()=>expect(action).toHaveBeenCalledWith(expect.objectContaining({action:"edit",itemId:"one",expectedRevision:2,instruction:"Edited"})));
 fireEvent.click(screen.getByRole("button",{name:"Move request 2 up"}));expect(action).toHaveBeenCalledWith(expect.objectContaining({action:"move",itemId:"two",expectedRevision:1,direction:"up"}));
 fireEvent.click(screen.getByRole("button",{name:"Stop current work, then run pending requests"}));expect(action).toHaveBeenCalledWith(expect.objectContaining({action:"stop_then_drain",runId:"run"}));
 expect(screen.getByText(/Reasoning high/)).toBeInTheDocument();
});
it("keeps queue immutable while stopping and permits cancellation of the pending drain",()=>{
 const action=vi.fn().mockResolvedValue(undefined);render(<SessionQueue queue={{...queue,control:{...queue.control,mode:"stop_then_drain"}}} currentRun={run} readOnly={false} busy={false} taskId="task" sessionId="session" onAction={action}/>);
 expect(screen.getAllByRole("button",{name:"Edit"})[0]).toBeDisabled();expect(screen.getByRole("status")).toHaveTextContent("confirmed termination");fireEvent.click(screen.getByRole("button",{name:"Cancel pending execution"}));expect(action).toHaveBeenCalledWith(expect.objectContaining({action:"cancel_drain"}));
});
it("retains an unsaved edit when its request is claimed before the user saves",()=>{
 const action=vi.fn().mockResolvedValue(undefined);const props={currentRun:run,readOnly:false,busy:false,taskId:"task",sessionId:"session",onAction:action};
 const view=render(<SessionQueue {...props} queue={queue}/>);fireEvent.click(screen.getAllByRole("button",{name:"Edit"})[0]);fireEvent.change(screen.getByLabelText("Edit pending request"),{target:{value:"Keep this unsaved text"}});
 view.rerender(<SessionQueue {...props} queue={{...queue,items:[]}}/>);
 expect(screen.getByLabelText("Retained edit draft")).toHaveValue("Keep this unsaved text");expect(screen.getByRole("status")).toHaveTextContent("was not applied");expect(action).not.toHaveBeenCalled();
});

it("shows the empty pending-request destination while work is running without offering an empty drain",()=>{
 render(<SessionQueue queue={{...queue,items:[]}} currentRun={run} readOnly={false} busy={false} taskId="task" sessionId="session" onAction={vi.fn()}/>);
 expect(screen.getByRole("region",{name:"Pending requests"})).toBeVisible();
 expect(screen.getByText("Add a request below to run after the current work.")).toBeVisible();
 expect(screen.queryByRole("button",{name:"Stop current work, then run pending requests"})).not.toBeInTheDocument();
});
