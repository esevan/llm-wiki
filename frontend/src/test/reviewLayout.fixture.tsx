// Full App and shared styles, with disposable in-memory review data only.
import { createRoot } from 'react-dom/client';
import { App } from '../app/App';
import { fixtureWorkspace } from '../features/workbench/referencePreviewFixtures';
import type { TaskAggregate } from '../types/taskWorkbench';
import '../theme/tokens.css';
import '../theme/legacy-components.css';
import '../theme/refresh.css';
import '../theme/conflict-review.css';
document.documentElement.lang = 'ko';
const task: TaskAggregate = {id:'layout-review',kind:'task',title:'오프라인 검색에서 정확한 출처 열기',state:'completed',taskRevision:1,detail:'네트워크 없이 저장된 문서와 정확한 버전을 확인합니다.',outcome:'원문과 검색 결과를 대조합니다.',workLog:[],checklist:[],decisions:[],readinessEntries:[],completion:{id:'completed-review',evidence:'임시 문서로 보관된 원문을 확인했습니다.'}};
const source = {type:'task_distillation',id:'claim',revision:'1',locator:'claims/result',quote:'보관된 문서와 정확한 버전을 확인했습니다.'};
const applicability = {summary:'네트워크 없이 자료를 확인할 때 활용합니다.',representativeQuestions:['보관된 원문이 맞는지 어떻게 확인할까요?'],helpsWith:['로컬 원문 확인'],conditions:['로컬에 해당 버전이 있어야 합니다.'],exclusions:['원격 동기화는 포함하지 않습니다.']};
const bodyMarkdown = '# 오프라인에서 원문 확인하기\n\n'+('검색 결과에 문서 식별자와 버전을 함께 남깁니다. 저장된 원문과 버전을 비교하고, 일치하는 원문이 없으면 확인이 필요하다고 알립니다.\n\n'.repeat(25));
const version = {revision:1,derivationKind:'generated',articleType:'guide',title:task.title,bodyMarkdown,contentHash:'layout-content',generationSnapshotHash:'layout-snapshot',freshness:'current',storedFreshness:'current',qualityState:'valid',modelStatus:'enhanced',modelError:'',createdAt:'2026-10-21T10:00:00Z',applicability,result:{article:{type:'guide',title:task.title,finalOutcomes:[{topicKey:'source',statement:'보관된 문서와 정확한 버전을 확인했습니다.',claimIds:['claim']}],bodyMarkdown,applicability:{...applicability,sourceRefs:[source]},claimBindings:[{claimId:'claim',statement:source.quote,epistemicState:'verified',sourceRefs:[source]}],assumptionBindings:[]},ideas:[],qualityFindings:[]}};
const workspace = fixtureWorkspace(2);
Object.assign(workspace.preview!.current!.fields,{title:task.title,description:task.detail,goal:task.outcome,completionCriteria:['네트워크를 끈 상태에서 원문을 열 수 있다.','같은 제목의 문서도 정확한 버전으로 구별한다.']});
window.llmWikiApplication = {request: async ({path}) => {
  const body = path === '/workbench' ? {revision:1,activeShortcuts:[],refiningShortcuts:[],categories:[{id:'general',label:'일반',items:[task]}]}
    : path.endsWith('/knowledge') ? {taskId:task.id,pointers:{currentPrivateRevision:1},versions:[version],ideas:[],reviewNotes:[]}
    : path.endsWith('/reference-workspace') ? workspace
    : path.endsWith('/refinement') ? {id:'layout-session',subjectKind:'task',subjectId:task.id,draftRevision:1,messages:[{id:'question',role:'user',body:'인터넷이 끊겨도 검색 결과의 원문을 확인하고 싶어.'},{id:'answer',role:'assistant',body:'정확한 문서와 버전 확인을 완료 기준에 넣었습니다.'}]}
    : path.endsWith('/lineage') ? {nodes:[],edges:[],unresolved:[]}
    : path.endsWith('/proposals') ? []
    : path.startsWith('/tasks/') ? task : {connections:[],sessions:[],items:[],jobs:[]};
  return {ok:true,status:200,json:async <T,>()=>body as T,text:async()=>'',body:null};
}};
createRoot(document.getElementById('root')!).render(<App />);
