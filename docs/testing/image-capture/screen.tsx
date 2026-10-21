window.fixtureErrors=[];window.addEventListener('error',event=>window.fixtureErrors.push(event.message));window.addEventListener('unhandledrejection',event=>window.fixtureErrors.push(String(event.reason)));
import {flushSync} from 'react-dom';
const runtime=import.meta.glob('../../../frontend/public/runtime/*.js',{query:'?raw',import:'default',eager:true});
import english from '../../../frontend/public/i18n/en.json';
import korean from '../../../frontend/public/i18n/ko.json';
// Full App and shared styles, with disposable in-memory review data only.
import { createRoot } from 'react-dom/client';
import { App } from '../../../frontend/src/app/App';
import { fixtureWorkspace } from '../../../frontend/src/features/workbench/referencePreviewFixtures';
import type { TaskAggregate } from '../../../frontend/src/types/taskWorkbench';
import '../../../frontend/src/theme/tokens.css';
import '../../../frontend/src/theme/legacy-components.css';
import '../../../frontend/src/theme/refresh.css';
import '../../../frontend/src/theme/conflict-review.css';
const en = new URLSearchParams(location.search).get('lang') === 'en';
document.documentElement.lang = en ? 'en' : 'ko';
const task: TaskAggregate = {id:'layout-review',kind:'task',title:'오프라인 검색에서 정확한 출처 열기',state:'completed',taskRevision:1,detail:'네트워크 없이 저장된 문서와 정확한 버전을 확인합니다.',outcome:'원문과 검색 결과를 대조합니다.',workLog:[],checklist:[],decisions:[],readinessEntries:[],completion:{id:'completed-review',evidence:'임시 문서로 보관된 원문을 확인했습니다.'}};
if(en) Object.assign(task,{title:'Open exact sources offline',detail:'Compare locally saved documents and exact versions without a network connection.',outcome:'Compare the source and search result.'});
const label=(enText:string,koText:string)=>en?enText:koText;
task.workLog=[{id:'demo-log',body:label('Compared the stored source and exact version with the search result. The check used synthetic documents only.','검색 결과와 저장된 출처·정확한 버전을 대조했습니다. 합성 문서만 사용해 확인했습니다.'),createdAt:'2026-10-21T10:00:00Z'}];
task.checklist=[{id:'demo-check',body:label('Open the exact saved source without a network connection','네트워크 없이 정확한 저장 원문 열기'),checked:true}];
const source = {type:'task_distillation',id:'claim',revision:'1',locator:'claims/result',quote:'보관된 문서와 정확한 버전을 확인했습니다.'};
const applicability = {summary:'네트워크 없이 자료를 확인할 때 활용합니다.',representativeQuestions:['보관된 원문이 맞는지 어떻게 확인할까요?'],helpsWith:['로컬 원문 확인'],conditions:['로컬에 해당 버전이 있어야 합니다.'],exclusions:['원격 동기화는 포함하지 않습니다.']};
const bodyMarkdown = '# 오프라인에서 원문 확인하기\n\n'+('검색 결과에 문서 식별자와 버전을 함께 남깁니다. 저장된 원문과 버전을 비교하고, 일치하는 원문이 없으면 확인이 필요하다고 알립니다.\n\n'.repeat(25));
let version = {revision:1,derivationKind:'generated',articleType:'guide',title:task.title,bodyMarkdown,contentHash:'layout-content',generationSnapshotHash:'layout-snapshot',freshness:'current',storedFreshness:'current',qualityState:'valid',modelStatus:'enhanced',modelError:'',createdAt:'2026-10-21T10:00:00Z',applicability,result:{article:{type:'guide',title:task.title,finalOutcomes:[{topicKey:'source',statement:'보관된 문서와 정확한 버전을 확인했습니다.',claimIds:['claim']}],bodyMarkdown,applicability:{...applicability,sourceRefs:[source]},claimBindings:[{claimId:'claim',statement:source.quote,epistemicState:'verified',sourceRefs:[source]}],assumptionBindings:[]},ideas:[],qualityFindings:[]}};
const workspace = fixtureWorkspace(2);
Object.assign(workspace.preview!.current!.fields,{title:task.title,description:task.detail,goal:task.outcome,completionCriteria:['네트워크를 끈 상태에서 원문을 열 수 있다.','같은 제목의 문서도 정확한 버전으로 구별한다.']});
// Localize authored fixture content as well as product chrome.
source.quote=label('The stored document and its exact version were checked.','보관된 문서와 정확한 버전을 확인했습니다.');
Object.assign(applicability,{summary:label('Use when reviewing saved evidence offline.','오프라인에서 저장된 근거를 검토할 때 활용합니다.'),representativeQuestions:[label('How can I verify the saved version?','저장된 버전은 어떻게 확인하나요?')],helpsWith:[label('Local source review','로컬 출처 검토')],conditions:[label('The exact source is saved locally.','정확한 원문이 로컬에 저장되어 있어야 합니다.')],exclusions:[label('Remote synchronization is outside this scope.','원격 동기화는 범위 밖입니다.')]});
version.bodyMarkdown='# '+label('Verify saved sources offline','오프라인에서 저장된 출처 확인')+'\n\n'+source.quote+'\n\n'+label('Compare the document identifier and version before reusing the result. Remote synchronization is outside this check.','결과를 재사용하기 전에 문서 식별자와 버전을 대조합니다. 원격 동기화는 이번 확인 범위 밖입니다.');
Object.assign(version.result.article,{bodyMarkdown:version.bodyMarkdown,applicability:{...applicability,sourceRefs:[source]},finalOutcomes:[{topicKey:'source',statement:source.quote,claimIds:['claim']}],claimBindings:[{claimId:'claim',statement:source.quote,epistemicState:'verified',sourceRefs:[source]}]});
Object.assign(workspace.preview!.current!.fields,{background:label('Saved sources must stay traceable offline.','저장된 출처를 오프라인에서도 추적할 수 있어야 합니다.'),scope:label('Local search and exact source review','로컬 검색과 정확한 출처 검토'),nonGoals:label('Remote synchronization','원격 동기화'),constraints:[label('Keep the document identifier and version.','문서 식별자와 버전을 유지합니다.')],completionCriteria:[label('Open the saved source offline.','저장된 원문을 오프라인에서 엽니다.'),label('Distinguish different versions with the same title.','같은 제목의 서로 다른 버전을 구별합니다.')],initialApproach:[label('Compare a synthetic source and search result.','합성 출처와 검색 결과를 대조합니다.')],assumptions:[]});
for(const ref of workspace.preview!.current!.references){ref.title=label('Saved source policy','저장된 출처 정책');ref.excerpt=source.quote;}
let proposed: typeof version.result | undefined;
const turns:Array<Record<string,unknown>>=[];
const replyText=()=>label('I preserved the exact quote and clarified the offline scope. Review the proposal before applying it.','정확한 인용을 보존하고 오프라인 범위를 명확히 했습니다. 제안을 검토한 뒤 적용하세요.');
window.llmWikiApplication = {request: async ({path,method,body:requestBody}) => {
  const input=JSON.parse(requestBody??'{}');
  let mutation:unknown;
  if(path.endsWith('/revision-chat')&&method==='POST'){
    proposed={...version.result,article:{...version.result.article,bodyMarkdown:version.bodyMarkdown+'\n\n'+label('If the saved version is absent, request the missing evidence before reuse.','저장된 버전이 없으면 재사용 전에 누락된 근거를 요청합니다.')}};
    const reply={id:input.operationId,baseRevision:version.revision,message:replyText(),missingEvidence:[],candidate:proposed,qualityState:'valid',diagnostics:[]};
    turns.push({id:input.operationId,userMessage:input.message,reply,state:'proposed',createdAt:'2026-10-21T10:00:00Z'});mutation=reply;
  }else if(path.includes('/revision-chat/')&&path.endsWith('/apply')&&proposed){version={...version,revision:2,bodyMarkdown:proposed.article.bodyMarkdown,result:proposed,contentHash:'layout-content-2'};Object.assign(turns[0],{state:'applied',appliedRevision:2});mutation={draftRevision:2};}
  if(mutation!==undefined)return {ok:true,status:200,json:async <T,>()=>mutation as T,text:async()=>'',body:null};
  const body = path.startsWith('/settings/locale') ? {locale:en?'en':'ko'} : path==='/i18n/en' ? english : path==='/i18n/ko' ? korean : path==='/provider/config' ? {base_url:'http://127.0.0.1:11434/v1',model:'local-demo',advanced_model:'local-review',api_key_configured:false,advanced_tasks:{},async_worker_count:2,request_timeout_seconds:180} : path==='/settings/vault' ? {path:'synthetic-vault'} : path==='/dashboard' ? {goals:[{title:en?'Keep decisions traceable':'결정의 근거를 보존하기'}],events:[]} : path==='/notifications' ? {notifications:[],unread_count:0} : path==='/jobs' ? {jobs:[{id:'demo-failed',task_kind:'embedding_refresh',entity_type:'vault',entity_id:'demo-vault',status:'failed',result_interface:'none',created_at:'2026-10-21T12:00:00Z',error:{message:en?'Synthetic indexing failure. Retry after checking the local source.':'합성 색인 실패입니다. 로컬 출처를 확인한 뒤 재시도하세요.'}}]} : path==='/transitions' ? {transitions:[]} : path.startsWith('/search?') ? {results:[{path:'knowledge/offline-search.md',title:en?'Verify sources offline':'오프라인 출처 확인',snippet:en?'Compare the saved document and its exact version.':'저장된 문서와 정확한 버전을 대조합니다.'}]} : path === '/workbench' ? {revision:1,activeShortcuts:[],refiningShortcuts:[],categories:[{id:'general',label:en?'General':'일반',items:[task,{...task,id:'active',state:'in_progress',title:en?'Verify offline source links':'오프라인 출처 링크 검증',completion:undefined},{...task,id:'held',state:'on_hold',title:en?'Review export format':'내보내기 형식 검토',completion:undefined}]}]}
    : path.endsWith('/knowledge') ? {taskId:task.id,pointers:{currentPrivateRevision:version.revision},versions:[version],ideas:[],reviewNotes:[]}
    : path.endsWith('/revision-chat') ? {baseRevision:version.revision,diagnostics:[],turns} : path.endsWith('/reference-workspace') ? workspace
    : path.endsWith('/refinement') ? {id:'layout-session',subjectKind:'task',subjectId:task.id,draftRevision:1,messages:[{id:'question',role:'user',body:label('I want to verify exact sources offline.','인터넷이 끊겨도 검색 결과의 원문을 확인하고 싶어.')},{id:'answer',role:'assistant',body:label('I included document and version checks in the completion criteria.','정확한 문서와 버전 확인을 완료 기준에 넣었습니다.')}]}
    : path.endsWith('/lineage') ? {nodes:[],edges:[],unresolved:[]}
    : path.endsWith('/proposals') ? []
    : path.startsWith('/tasks/') ? task : {connections:[],sessions:[],items:[],jobs:[]};
  return {ok:true,status:200,json:async <T,>()=>body as T,text:async()=>'',body:null};
}};
flushSync(()=>createRoot(document.getElementById('root')!).render(<App />));
window.llmWikiFormatSystemTime=(value)=>value;
for(const name of ['foundation','jobs','workbench','conflicts','explore','work-tracking','manual','search-settings','solution-work','archive','transitions','completed-workspace']){const script=document.createElement('script');script.textContent=runtime['../../../frontend/public/runtime/'+name+'.js'] as string;document.body.append(script);}
setTimeout(()=>{const view=new URLSearchParams(location.search).get('view');if(view)document.querySelector<HTMLButtonElement>('[data-view="'+view+'"]')?.click();if(view==='ai-setup')setTimeout(()=>{document.querySelector('#provider-url')?.scrollIntoView({block:'start'});window.scrollBy(0,-70)},250);if(view==='queue')document.querySelector<HTMLButtonElement>('#queue-toggle')?.click();if(view==='search'){(document.querySelector('#query') as HTMLInputElement).value=en?'offline':'오프라인';document.querySelector<HTMLFormElement>('#search-form')?.requestSubmit();}},1000);

const waitFor=async(selector:string)=>{for(let i=0;i<80;i++){const el=document.querySelector<HTMLElement>(selector);if(el)return el;await new Promise(r=>setTimeout(r,50));}throw Error('Capture target missing: '+selector);};
const mode=new URLSearchParams(location.search).get('view');
if(['task','preview','knowledge','knowledge-compare','knowledge-applied'].includes(mode??''))void(async()=>{
 (await waitFor('[data-control="workbench-completed-open"]')).click();
 (await waitFor('[data-control="task-detail-tab-'+(mode==='task'?'work':mode==='preview'?'details':'review')+'"]')).click();
 if(mode?.startsWith('knowledge')){
  const input=await waitFor('[data-control="knowledge-chat-input"]') as HTMLTextAreaElement;
  const message=label('Clarify what to do when the exact saved version is missing, preserving the source quote.','정확한 저장 버전이 없을 때 할 일을 명확히 하고 출처 인용은 보존해 주세요.');
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value')!.set!.call(input,message);input.dispatchEvent(new Event('input',{bubbles:true}));
  if(mode!=='knowledge'){
   await new Promise(r=>setTimeout(r,100));(await waitFor('[data-control="knowledge-chat-send"]')).click();
   const apply=await waitFor('[data-control="knowledge-chat-apply"]');
   if(mode==='knowledge-applied')apply.click();
  }
 }
 await new Promise(r=>setTimeout(r,200));
 window.workspaceDiagnostics={mode,taskDefinitions:document.querySelectorAll('.task-definition-read').length,previewChat:document.querySelector('.refinement-conversation')?.getBoundingClientRect().width,knowledgeChat:document.querySelector('.knowledge-revision-chat')?.getBoundingClientRect().width};
})().catch(e=>window.fixtureErrors.push(String(e)));
