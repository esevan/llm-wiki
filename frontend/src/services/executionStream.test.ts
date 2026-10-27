import { describe, expect, it } from "vitest";
import { applyExecutionLive, mergeExecutionSnapshot } from "./executionStream";
import type { TaskExecutionRun, TaskExecutionSnapshot } from "../types/taskWorkbench";
const run = {id:"run",taskId:"task",sessionId:"session",status:"running",revision:1,formalRequests:[],evidence:[]} as unknown as TaskExecutionRun;
const snapshot = {revision:1000000001,liveRevision:1,runs:[run],selectedRun:run,activeRunId:run.id} as TaskExecutionSnapshot;
describe("execution stream revisions and terminal tails",()=>{
 it("replaces only the identified active run and rejects duplicate/stale/unknown live events",()=>{
  const event={kind:"live" as const,runId:"run",revision:1000000002,liveRevision:2,liveStatus:{kind:"agentMessage",status:"running" as const,text:"tail"}};
  const next=applyExecutionLive(snapshot,event)!;
  expect(next.runs[0].liveStatus?.text).toBe("tail");expect(next.selectedRun).toBe(next.runs[0]);
  expect(applyExecutionLive(next,event)).toBe(next);
  expect(applyExecutionLive(snapshot,{...event,runId:"other"})).toBe(snapshot);
  expect(mergeExecutionSnapshot(next,snapshot)).toBe(next);
 });
 for(const status of ["succeeded","failed","cancelled","interrupted","needs_attention"] as const)it(`preserves flushed fragments on ${status} without overriding authoritative text`,()=>{
  const live={...run,liveStatus:{kind:"agentMessage",status:"running" as const,text:"Full final tail"}};
  const prior={...snapshot,runs:[live],selectedRun:live};
  const terminal={...run,status};const incoming={...snapshot,revision:2000000002,runs:[terminal],selectedRun:terminal,activeRunId:null};
  const merged=mergeExecutionSnapshot(prior,incoming);expect(merged.runs[0].liveStatus?.text).toBe("Full final tail");
  expect(applyExecutionLive(merged,{kind:"live",runId:"run",revision:2000000003,liveRevision:3,liveStatus:{kind:"agentMessage",status:"running",text:"stale"}})).toBe(merged);
  const authoritative={...terminal,finalReport:"Authoritative complete answer"};expect(mergeExecutionSnapshot(prior,{...incoming,runs:[authoritative],selectedRun:authoritative}).runs[0].liveStatus).toBeUndefined();
 });
});
