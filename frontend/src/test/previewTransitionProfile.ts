// Isolated packaged-fixture profiling. These counters describe DOM mutations and
// paint opportunities, not React commits; the Vitest Profiler test measures those.
import { invoke } from '@tauri-apps/api/core';
import type { ReferenceWorkspace } from '../types/taskWorkbench';
import type { WorkbenchScenarioHarness } from './workbenchScenarios';

export async function profilePreviewTransition(h:WorkbenchScenarioHarness,providerUrl:string){
  await h.api('/provider/config','PUT',{base_url:providerUrl,model:'deterministic',api_key:'desktop-e2e-key'});
  const previousLocale=document.documentElement.lang;
  for(const locale of ['en','ko']) for(const width of [1200,900]){
    document.documentElement.lang=locale;
    await invoke('desktop_e2e_resize_window',{width,height:820});
    const title=`Preview profile ${locale} ${width} ${Date.now()}`;
    await h.create(title);const task=await h.task(title);
    await h.api(`/tasks/${task.id}/revisions`,'POST',{expectedTaskRevision:task.taskRevision,patch:{detail:'사용자 조사에서 수집한 조건과 판단 기준을 비교합니다. Review the gathered conditions and decision criteria. '.repeat(12),outcome:'다음 작업에서 재사용할 검토 문서 / A reusable review',scope:'기록된 조사 / Recorded research',nonGoals:'실행이나 발행 / Execution or publication',validationCriteria:'조건과 근거 보존 / Preserve conditions and rationale'}});
    const original=window.llmWikiApplication.request;
    let release!:()=>void;const held=new Promise<void>(resolve=>{release=resolve;});
    let readyAt=0;let released=false;
    window.llmWikiApplication.request=async request=>{
      const response=await original(request);
      if(request.path.endsWith('/reference-workspace')&&response.ok){
        const payload=await response.json<ReferenceWorkspace>();
        if(payload.preview?.current&&!released){readyAt ||= performance.now();await held;}
        return {...response,json:async<T,>()=>payload as T};
      }
      return response;
    };
    try{
      await h.detail(title);
      const tab=document.querySelector<HTMLButtonElement>('[data-control="task-detail-tab-details"]')!;
      await h.prepareClick(tab,'Open manual definition before generated preview');h.click(tab,'Open manual definition before generated preview');
      await h.waitFor(()=>Boolean(document.querySelector('[data-control="task-definition-fallback"]')),'manual preview surface');
      await h.waitFor(()=>readyAt>0,'held AI preview response');
      const manual=document.querySelector<HTMLElement>('[data-control="task-definition-fallback"]');
      if(!manual)throw new Error('Manual Task form was replaced while the canonical AI preview response was still held; check mirrored legacy proposals');
      const panel=document.querySelector<HTMLElement>('.refinement-panel')!;
      const pane=document.querySelector<HTMLElement>('.refinement-preview')!;
      const grid=document.querySelector<HTMLElement>('.refinement-workspace')!;
      const composer=panel.querySelector<HTMLTextAreaElement>('[data-control="refinement-message"]')!;
      h.enter(composer,'Unsent question / 작성 중인 질문');composer.focus({preventScroll:true});
      pane.scrollTop=Math.min(120,Math.max(0,pane.scrollHeight-pane.clientHeight));
      const before={viewportHeight:pane.clientHeight,scrollHeight:pane.scrollHeight,panelWidth:panel.getBoundingClientRect().width,panelHeight:panel.getBoundingClientRect().height,bodyHeight:manual.getBoundingClientRect().height,columns:getComputedStyle(grid).gridTemplateColumns,scroll:pane.scrollTop};
      let mutations=0,removedManual=0,resizeChanges=0;const mounts=new Set<Element>();
      const observer=new MutationObserver(records=>{
        mutations++;
        for(const record of records){
          for(const node of record.removedNodes)if(node===manual||(node instanceof Element&&node.contains(manual)))removedManual++;
          for(const node of record.addedNodes)if(node instanceof Element){
            if(node.matches('.reference-aware-preview:not(.task-definition-fallback)'))mounts.add(node);
            node.querySelectorAll('.reference-aware-preview:not(.task-definition-fallback)').forEach(element=>mounts.add(element));
          }
        }
      });observer.observe(pane,{subtree:true,childList:true,attributes:true,characterData:true});
      const last=new Map<Element,string>();const resize=new ResizeObserver(entries=>{for(const entry of entries){const size=`${Math.round(entry.contentRect.width)}:${Math.round(entry.contentRect.height)}`;if(last.has(entry.target)&&last.get(entry.target)!==size)resizeChanges++;last.set(entry.target,size);}});
      resize.observe(panel);resize.observe(pane);resize.observe(grid);
      const waitForFrames=<T,>(label:string,promise:Promise<T>):Promise<T>=>new Promise((resolve,reject)=>{
        const timer=setTimeout(()=>reject(new Error(`${label}: requestAnimationFrame unavailable for 5s (visibility=${document.visibilityState}, focus=${document.hasFocus()}); paint timing cannot be measured`)),5000);
        promise.then(value=>{clearTimeout(timer);resolve(value);},error=>{clearTimeout(timer);reject(error);});
      });
      const baselineStart=performance.now();
      await waitForFrames('Baseline',new Promise<void>(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve()))));
      const baselineTwoFrameMs=performance.now()-baselineStart;
      const start=performance.now();released=true;release();
      let frames=0,paintAt=0;
      await waitForFrames('Preview paint',new Promise<void>((resolve,reject)=>{
        const sample=()=>{
          frames++;
          if(pane.querySelector('[data-control="reference-preview-apply"]'))requestAnimationFrame(()=>{paintAt=performance.now();resolve();});
          else if(performance.now()-start>5000)reject(new Error('Generated preview did not paint after bridge release'));
          else requestAnimationFrame(sample);
        };requestAnimationFrame(sample);
      }));
      observer.disconnect();resize.disconnect();
      const ai=pane.querySelector<HTMLElement>('.reference-aware-preview:not(.task-definition-fallback)')!;
      const after={viewportHeight:pane.clientHeight,scrollHeight:pane.scrollHeight,panelWidth:panel.getBoundingClientRect().width,panelHeight:panel.getBoundingClientRect().height,bodyHeight:ai.getBoundingClientRect().height,columns:getComputedStyle(grid).gridTemplateColumns,scroll:pane.scrollTop};
      const result={locale,width,baselineTwoFrameMs:+baselineTwoFrameMs.toFixed(2),bridgeReleaseToPaintMs:+(paintAt-start).toFixed(2),heldResponseMs:+(start-readyAt).toFixed(2),animationFrames:frames,domMutationBatches:mutations,manualSubtreeRemovals:removedManual,aiSubtreeMounts:mounts.size,resizeChanges,before,after,panelPreserved:document.querySelector('.refinement-panel')===panel,composerPreserved:panel.querySelector('[data-control="refinement-message"]')===composer,unsentPreserved:composer.value==='Unsent question / 작성 중인 질문'};
      if(!result.panelPreserved||!result.composerPreserved||!result.unsentPreserved||mounts.size!==1)throw new Error(`Preview identity regression: ${JSON.stringify(result)}`);
      await h.step(`PreviewTransition WKWebView fixture ${JSON.stringify(result)}`);
    }finally{released=true;release();window.llmWikiApplication.request=original;}
    const close=document.querySelector<HTMLElement>('[data-control="task-detail-close"]')!;await h.prepareClick(close,'Close profiling fixture');h.click(close,'Close profiling fixture');
    await h.waitFor(()=>!document.querySelector('.task-detail'),'closed profiling fixture');
  }
  document.documentElement.lang=previousLocale;
}
