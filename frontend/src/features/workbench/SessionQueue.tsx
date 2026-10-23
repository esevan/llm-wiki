import { useEffect, useState } from "react";
import type { TaskQueue, TaskExecutionRun } from "../../types/taskWorkbench";
import type { taskExecutionClient } from "../../services/taskExecutionClient";
type Action = Parameters<typeof taskExecutionClient.queue>[0];
export function SessionQueue({queue, currentRun, readOnly, busy, taskId, sessionId, onAction, editorDraft, onEditorChange}: {editorDraft?:{id:string;body:string}; onEditorChange?:(value:{id:string;body:string}|undefined)=>void; queue?:TaskQueue;currentRun?:TaskExecutionRun;readOnly:boolean;busy:boolean;taskId:string;sessionId:string;onAction:(input:Action)=>Promise<void>}) {
  const ko=document.documentElement.lang.startsWith("ko");
  const [editing,setEditing]=useState<string | undefined>(editorDraft?.id);const [body,setBody]=useState(editorDraft?.body ?? "");
  useEffect(()=>{onEditorChange?.(editing?{id:editing,body}:undefined);},[editing,body,onEditorChange]);
  if(!queue || (!queue.items.length && !editing))return null;
  const waiting=queue.control.mode==="stop_then_drain";
  const action=(input:Omit<Action,"taskId"|"sessionId">)=>onAction({...input,taskId,sessionId});
  return <section className="session-queue" aria-label={ko?"대기 요청":"Pending requests"}>
    <header><h4>{ko?"대기 요청":"Pending requests"} <span>{queue.items.length}</span></h4><span>{queue.control.mode==="active"?(ko?"현재 실행 완료 뒤 순서대로 실행":"Run in order after current work finishes"):(ko?"대기 중 · 자동 시작 안 함":"Waiting · automatic start paused")}</span></header>
    <p className="session-queue-meta">{ko?"각 요청에 기록한 설정으로 실행 · 세션 기본값 변경 영향 없음":"Uses each request’s recorded settings · later defaults do not alter pending work"}</p>
    <ol>{queue.items.map((item,index)=><li key={item.id}>
      <span className="session-queue-number">{index+1}</span><div>{editing===item.id?<><label>{ko?"대기 요청 편집":"Edit pending request"}<textarea data-control="session-queue-edit-body" value={body} onChange={e=>setBody(e.target.value)}/></label><button data-control="session-queue-edit-save" disabled={busy||!body.trim()} onClick={()=>void action({action:"edit",itemId:item.id,expectedRevision:item.revision,instruction:body}).then(()=>setEditing(undefined)).catch(()=>{})}>{ko?"변경 저장":"Save changes"}</button><button data-control="session-queue-edit-cancel" onClick={()=>setEditing(undefined)}>{ko?"취소":"Cancel"}</button></>:<p>{item.instruction}</p>}
        <small>{item.settings.cwd} · {item.settings.model} · {ko?"추론":"Reasoning"} {item.settings.resolvedEffort} {item.settings.reasoningEffort?"":(ko?"(상속값 확정)":"(resolved inherited value)")} · {item.attachment?.name??(ko?"첨부 없음":"No attachment")}</small>
      </div><div className="session-queue-controls">
        <button data-control="session-queue-edit" disabled={readOnly||busy||waiting} onClick={()=>{setEditing(item.id);setBody(item.instruction);}}>{ko?"편집":"Edit"}</button>
        <button data-control="session-queue-up" aria-label={ko?`${index+1}번 요청 위로`:`Move request ${index+1} up`} disabled={readOnly||busy||waiting||index===0} onClick={()=>void action({action:"move",itemId:item.id,expectedRevision:item.revision,direction:"up"}).catch(()=>{})}>↑</button>
        <button data-control="session-queue-down" aria-label={ko?`${index+1}번 요청 아래로`:`Move request ${index+1} down`} disabled={readOnly||busy||waiting||index===queue.items.length-1} onClick={()=>void action({action:"move",itemId:item.id,expectedRevision:item.revision,direction:"down"}).catch(()=>{})}>↓</button>
        <button data-control="session-queue-remove" disabled={readOnly||busy||waiting} onClick={()=>void action({action:"remove",itemId:item.id,expectedRevision:item.revision}).catch(()=>{})}>{ko?"삭제":"Remove"}</button>
      </div>
    </li>)}</ol>
    {editing && !queue.items.some(item=>item.id===editing) && <section>
      <p role="status">{ko?"편집하던 요청이 이미 실행되었거나 삭제되었습니다. 이 초안은 실행에 적용되지 않았습니다.":"The request being edited has already started or was removed. This draft was not applied to execution."}</p>
      <label>{ko?"보존된 편집 초안":"Retained edit draft"}<textarea readOnly value={body} data-control="session-queue-retained-edit"/></label>
      <button data-control="session-queue-edit-cancel" onClick={()=>setEditing(undefined)}>{ko?"편집 닫기":"Close edit"}</button>
    </section>}
    {waiting&&<p role="status">{ko?"현재 작업 종료를 확인하고 있습니다. 종료가 확인되기 전에는 대기 요청을 시작하지 않습니다.":"Waiting for confirmed termination. Pending requests will not start before the current Run ends."}</p>}
    {queue.control.reason&&<p role="status">{queue.control.reason}</p>}
    {queue.items.length>0 && <footer>{waiting?<button data-control="session-queue-cancel-drain" disabled={busy||readOnly} onClick={()=>void action({action:"cancel_drain"}).catch(()=>{})}>{ko?"대기 실행 취소":"Cancel pending execution"}</button>:<>
      {currentRun&&["running","awaiting_response"].includes(currentRun.status)&&<button data-control="session-queue-stop-drain" disabled={busy||readOnly||currentRun.stopRequested} onClick={()=>void action({action:"stop_then_drain",runId:currentRun.id}).catch(()=>{})}>{ko?"현재 작업 중단 후 대기 요청 실행":"Stop current work, then run pending requests"}</button>}
      <button data-control="session-queue-toggle" disabled={busy||readOnly} onClick={()=>void action({action:queue.control.mode==="active"?"pause":"resume"}).catch(()=>{})}>{queue.control.mode==="active"?(ko?"대기 실행 일시정지":"Pause pending execution"):(ko?"대기 실행 계속":"Continue pending execution")}</button>
    </>}</footer>}
  </section>;
}
