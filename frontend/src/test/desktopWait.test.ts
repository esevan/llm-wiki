import {afterEach,describe,expect,it,vi} from "vitest";
import {waitFor,waitForAsync} from "./desktopWait";
afterEach(()=>vi.restoreAllMocks());
describe("desktop condition waits",()=>{
  it.each([false,true])("rechecks a satisfied condition after a suspended timer (async=%s)",async asynchronous=>{
    let clock=0;
    let ready=false;
    let wake!:()=>void;
    vi.spyOn(performance,"now").mockImplementation(()=>clock);
    vi.spyOn(window,"setTimeout").mockImplementation(callback=>{wake=callback as ()=>void;return 1;});
    const pending=asynchronous?waitForAsync(async()=>ready,"committed view"):waitFor(()=>ready,"committed view");
    await Promise.resolve();
    clock=23_000;
    ready=true;
    wake();
    await expect(pending).resolves.toBeUndefined();
  });
  it.each([false,true])("still rejects an unsatisfied condition after the deadline (async=%s)",async asynchronous=>{
    let clock=0;
    let wake!:()=>void;
    vi.spyOn(performance,"now").mockImplementation(()=>clock);
    vi.spyOn(window,"setTimeout").mockImplementation(callback=>{wake=callback as ()=>void;return 1;});
    const pending=asynchronous?waitForAsync(async()=>false,"missing view"):waitFor(()=>false,"missing view");
    const assertion=expect(pending).rejects.toThrow("Timed out waiting for missing view");
    await Promise.resolve();clock=23_000;wake();await assertion;
  });
});
