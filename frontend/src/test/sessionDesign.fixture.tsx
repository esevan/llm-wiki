// Render the shipping session component with synthetic data; never contact native storage or a provider.
import { createRoot } from "react-dom/client";
import { TaskWorkSessions } from "../features/workbench/TaskWorkSessions";
import type { TaskAggregate, TaskExecutionSnapshot, TaskWorkSession } from "../types/taskWorkbench";
import "../theme/tokens.css";
import "../theme/legacy-components.css";
import "../theme/refresh.css";
import "../features/workbench/task-workbench.css";
import "../../public/assets/fonts.css";
const q=new URLSearchParams(location.search);document.documentElement.lang=q.get("lang")||"ko";const ko=document.documentElement.lang==="ko";const state=q.get("state")||"running";
const task={id:"review-task",kind:"task",title:ko?"동기화 실패 원인 확인하기":"Inspect reconnect synchronization failure",state:state==="completed"?"completed":"in_progress",taskRevision:1,detail:ko?"재현 조건을 확인하고 수정 방향을 정리합니다.":"Inspect reproduction conditions and propose a fix.",outcome:"",completedAt:state==="completed"?"2026-10-23T15:00:00Z":undefined} as TaskAggregate;
let session:TaskWorkSession={id:"review-session",taskId:task.id,title:"Session 02",displayLabel:ko?"동기화 오류 재현 조건 확인":"Inspect synchronization reproduction",provider:"codex",model:"gpt-5.6-sol",approvalMode:"ask",approvalsReviewer:"user",workspacePath:"/workspace/llm-wiki",reasoningEffort:"high",createdAt:"2026-10-23T14:32:00Z",updatedAt:"2026-10-23T14:32:00Z"};
const run={id:"run",taskId:task.id,sessionId:session.id,instruction:ko?"오프라인에서 다시 연결하면 동기화가 실패합니다. 재현 조건과 관련 코드를 확인해 주세요.":"Synchronization fails after reconnecting. Inspect reproduction conditions and related code.",status:state==="completed"?"succeeded":"running",stopRequested:false,provider:"codex",model:session.model,workspacePath:session.workspacePath,evidence:[],formalRequests:[],workLogSyncState:"synced",revision:1,startedAt:session.createdAt,finalReport:state==="completed"?(ko?"연결 상태 참조 경로를 확인했습니다. 실제 환경 재현은 확인하지 못했습니다.":"Inspected connection state references. Reproduction on a real device remains unverified."):undefined,liveStatus:state==="running"?{kind:"agentMessage",status:"running",text:ko?"재연결 처리와 재시도 경로를 확인하고 있습니다.":"Inspecting reconnect handling and retry paths."}:undefined};
let snapshot={revision:1,runs:state==="environment"?[]:[run],activeRunId:state==="running"?"run":null,selectedRun:state==="environment"?null:run,effectiveConfig:{model:session.model,cwd:session.workspacePath,approvalPolicy:"on-request",approvalsReviewer:"user",sandbox:"workspace-write",provenance:state==="environment"?"preflight":"bound_thread",settingsRevision:"fixture-settings",ready:true,reasoningEffort:"high",capabilities:{structuredUserInput:true}},queue:{control:{mode:"active",reason:"",revision:1},items:state==="running"?[{id:"q1",instruction:ko?"확인한 원인을 바탕으로 수정안을 정리해 줘.":"Propose a fix based on the cause.",position:1,revision:1,settings:{model:session.model,cwd:session.workspacePath,reasoningEffort:"high",resolvedEffort:"high",approvalsReviewer:"user"}},{id:"q2",instruction:ko?"관련 테스트와 남은 위험도 확인해 줘.":"Review related tests and remaining risks.",position:2,revision:1,settings:{model:session.model,cwd:session.workspacePath,reasoningEffort:null,resolvedEffort:"low",approvalsReviewer:"user"}}]:[]}} as TaskExecutionSnapshot;
// Additional synthetic review states exercise the shipping conditional DOM.
const activeStates=["running","approval","input","stop"];
if(["setup","ready","environment","held","empty","environment-error"].includes(state)){
 snapshot={...snapshot,runs:[],selectedRun:null,activeRunId:null,queue:{control:{mode:"paused",reason:"",revision:1},items:[]}};
}
if(state==="held")task.state="on_hold";
if(state==="setup"||state==="environment-error"){
 snapshot={...snapshot,effectiveConfig:undefined};
 if(state==="setup")session.workspacePath="";
 else session.workspacePath="/workspace/missing-project";
}
if(activeStates.includes(state)){
 snapshot={...snapshot,activeRunId:"run",runs:[{...snapshot.runs[0],status:state==="approval"||state==="input"?"awaiting_response":"running",stopRequested:state==="stop"}],queue:{control:{mode:state==="stop"?"stop_then_drain":"active",reason:"",revision:1},items:state==="running"?snapshot.queue!.items:[]}};
 if(state==="stop")snapshot.queue!.items=[{id:"pending-stop",instruction:ko?"작업 종료 확인 후 테스트해 줘.":"Test after termination is confirmed.",position:1,revision:1,settings:{model:session.model,cwd:session.workspacePath,resolvedEffort:"high",approvalsReviewer:"user"}}];
 const current=snapshot.runs[0];
 if(state==="approval")current.formalRequests=[{id:"approval",kind:"command_approval",status:"pending",isBlocking:true,title:ko?"검증 명령 실행을 허용할까요?":"Allow the validation command?",prompt:"npm test",choices:[{value:"accept",label:ko?"이번에 허용":"Allow once"},{value:"decline",label:ko?"거부":"Decline"}],questions:[]}];
 if(state==="input")current.formalRequests=[{id:"input",kind:"user_input",status:"pending",isBlocking:true,title:ko?"어떤 재현 환경을 확인할까요?":"Which reproduction environment?",prompt:ko?"확인할 환경을 선택해 주세요.":"Choose an environment.",choices:[],questions:[{id:"platform",header:ko?"재현 환경":"Environment",prompt:ko?"기준 환경을 선택하세요.":"Choose the target environment.",options:[{value:"mac",label:"macOS"},{value:"windows",label:"Windows"}],allowOther:true,isSecret:false}]}];
 snapshot.selectedRun=current;
}
if(state==="uncertain"||state==="queue-pause"){
 const current={...snapshot.runs[0],status:state==="uncertain"?"needs_attention" as const:"failed" as const};
 snapshot={...snapshot,runs:[current],selectedRun:current,activeRunId:null,queue:{control:{mode:state==="uncertain"?"reconcile_needed":"paused",reason:ko?"이전 실행을 확인한 뒤 계속하세요.":"Review the previous execution before continuing.",revision:1},items:[{id:"paused",instruction:ko?"다음 확인을 진행해 줘.":"Continue the next check.",position:1,revision:1,settings:{model:session.model,cwd:session.workspacePath,resolvedEffort:"high",approvalsReviewer:"user"}}]}};
}
window.llmWikiApplication={request:async({path,method,body})=>{
 let value:unknown=path.endsWith("work-sessions")?{sessions:state==="empty"?[]:[session]}:{session,entries:[]};
 if(method==="PUT"){session={...session,...JSON.parse(body||"{}")};value=session;}
 return {ok:true,status:200,json:async<T,>()=>value as T,text:async()=>"",body:null};
}};
let callback=0;
(window as unknown as {__TAURI_INTERNALS__:object}).__TAURI_INTERNALS__={transformCallback:()=>++callback,unregisterCallback:()=>{},invoke:async(cmd:string,args:{input?:Record<string,unknown>})=>{
 if(cmd==="task_session_external_thread_read")return {status:404,body:{error:{message:"Synthetic conversation has no imported transcript"}}};
 if(cmd==="task_session_prepare"&&state==="environment-error")return {status:400,body:{error:{message:ko?"invalid_folder: 존재하는 프로젝트 폴더를 선택하세요.":"invalid_folder: choose an existing project folder"}}};
 if(cmd==="task_session_models")return {status:200,body:{models:[{model:"gpt-5.6-sol",displayName:"GPT-5.6-Sol",supportedReasoningEfforts:["low","medium","high","ultra"].map(reasoningEffort=>({reasoningEffort}))},{model:"gpt-5.6-luna",displayName:"GPT-5.6-Luna",supportedReasoningEfforts:["low","medium","high"].map(reasoningEffort=>({reasoningEffort}))}]}};
 if(cmd==="task_session_queue"&&args.input){const i=args.input;if(i.action==="remove")snapshot={...snapshot,queue:{...snapshot.queue!,items:snapshot.queue!.items.filter(v=>v.id!==i.itemId)}};}
 snapshot={...snapshot,revision:(snapshot.revision??0)+1};return {status:200,body:snapshot};
}};
createRoot(document.getElementById("root")!).render(<main style={{maxWidth:q.get("width")==="narrow"?560:1248,margin:"24px auto",padding:20,background:"var(--color-bg-primary)",fontFamily:"var(--font-family-body)"}}><p style={{fontSize:12,color:"var(--color-text-secondary)"}}>SHIPPING COMPONENT REVIEW · synthetic data / no provider execution</p><h2>{task.title}</h2><TaskWorkSessions task={task}/></main>);
if(state==="environment"){
 const timer=window.setInterval(()=>{const details=document.querySelector<HTMLDetailsElement>(".task-session-settings");if(!details)return;clearInterval(timer);details.open=true;document.querySelector<HTMLButtonElement>('[data-control="task-session-model-refresh"]')?.click();},50);
}
