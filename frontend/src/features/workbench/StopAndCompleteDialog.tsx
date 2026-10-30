import { useEffect, useRef, useState } from "react";
import { taskCompletionClient, type CompletionReview, type CompletionProgress } from "../../services/taskExecutionClient";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export function StopAndCompleteDialog({ taskId, revision, title, evidence, onComplete, onClose }: {
  taskId: string; revision: number; title: string; evidence: string; onComplete: () => void; onClose: () => void;
}) {
  const text = useTaskWorkbenchText();
  const [review,setReview]=useState<CompletionReview>();
  const [progress,setProgress]=useState<CompletionProgress>();
  const [error,setError]=useState("");
  const [busy,setBusy]=useState(false);
  const dialog=useRef<HTMLDialogElement>(null), cancel=useRef<HTMLButtonElement>(null);
  const operation=useRef<string>(crypto.randomUUID()), pending=useRef(false), started=useRef(false), disposed=useRef(false), cancelling=useRef(false);
  const completed=useRef(onComplete), closed=useRef(onClose); completed.current=onComplete;closed.current=onClose;
  useEffect(()=>{
    disposed.current=false;
    const opener=document.activeElement as HTMLElement|null;const element=dialog.current;element?.showModal();cancel.current?.focus();
    let stopped=false;
    void taskCompletionClient.preview(taskId).then(async value=>{
      if(stopped)return;setReview(value);
      if(value.activeOperationId){
        operation.current=value.activeOperationId;started.current=true;
        const current=await taskCompletionClient.control({action:"poll",taskId,operationId:operation.current});
        if(stopped)return;setProgress(current);setError((current.issues??[]).join("\n"));
        if(current.state==="completed")completed.current();
      }
    }).catch(cause=>{if(!disposed.current)setError(String(cause));});
    return ()=>{stopped=true;disposed.current=true;element?.close();if(opener?.isConnected)opener.focus();};
  },[taskId]);
  const receive=(value:CompletionProgress)=>{
    if(disposed.current||cancelling.current)return;
    setProgress(value);setReview(value.review);setError((value.issues??[]).join("\n"));
    if(value.state==="completed")completed.current();
    if(value.state==="cancelled")closed.current();
  };
  const run=async(action:"begin"|"poll")=>{
    if(pending.current||cancelling.current||!review)return;
    pending.current=true;setBusy(true);setError("");started.current=true;
    try {receive(await taskCompletionClient.control({action,taskId,operationId:operation.current,...(action==="begin"?{expectedTaskRevision:revision,reviewHash:review.reviewHash,evidence}:{})}));}
    catch(cause){
      if(disposed.current||cancelling.current)return;
      const message=String(cause);
      if(message.includes("completion_review_stale")){
        started.current=false;setProgress(undefined);operation.current=crypto.randomUUID();
        setReview(await taskCompletionClient.preview(taskId));setError(text.stopCompleteStale);
      }else setError(message);
    }finally{pending.current=false;if(!disposed.current)setBusy(false);}
  };
  const runRef=useRef(run);runRef.current=run;
  useEffect(()=>{
    if(progress?.state!=="stopping"||busy||error)return;
    const timer=setTimeout(()=>void runRef.current("poll"),2000);return()=>clearTimeout(timer);
  },[progress,busy,error]);
  const close=async()=>{
    if(cancelling.current)return;
    if(!started.current){closed.current();return;}
    cancelling.current=true;setBusy(true);
    try{
      const result=await taskCompletionClient.control({action:"cancel",taskId,operationId:operation.current});
      if(result.state==="completed")completed.current();else closed.current();
    }catch(cause){cancelling.current=false;setError(String(cause));setBusy(false);}
  };
  return <dialog ref={dialog} className="workbench-delete-dialog task-stop-complete-dialog" aria-labelledby="stop-complete-title" aria-describedby="stop-complete-description" onCancel={event=>{event.preventDefault();void close();}}>
    <h2 id="stop-complete-title">{text.stopCompleteTitle}</h2><p><strong>{title}</strong></p>
    <p id="stop-complete-description">{text.stopCompleteExplanation}</p>
    {!review&&!error&&<p role="status">…</p>}
    {review&&<>
      <h3>{text.stopCompleteRuns} · {review.runs.length}</h3><ul>{review.runs.map(run=><li key={run.id}>{run.instruction}</li>)}</ul>
      <h3>{text.stopCompleteQueue} · {review.queued.length}</h3><ul>{review.queued.map(item=><li key={item.id}>{item.instruction}</li>)}</ul>
      {review.responses.length>0&&<><h3>{text.stopCompleteResponses}</h3><ul>{review.responses.map(item=><li key={item.id}>{item.title}</li>)}</ul></>}
    </>}
    {progress?.state==="stopping"&&<p role="status">{text.stopCompleteWaiting} ({progress.remaining.runs.length})</p>}
    {error&&<p role="alert">{error}</p>}
    {review?.activeEvidence&&<blockquote>{review.activeEvidence}</blockquote>}
    <p>{text.stopCompleteCancelHint}</p>
    <footer><button ref={cancel} type="button" data-control="task-stop-complete-cancel" onClick={()=>void close()}>{text.stopCompleteCancel}</button>
      <button type="button" data-control="task-stop-complete-confirm" disabled={!review||busy||review.taskRevision!==revision} onClick={()=>void run(progress?"poll":"begin")}>{progress?text.stopCompleteRetry:text.stopCompleteConfirm}</button></footer>
  </dialog>;
}
