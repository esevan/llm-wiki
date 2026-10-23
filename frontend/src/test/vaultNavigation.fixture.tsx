// Isolated visual fixture: shipping SearchView, Sidebar, fonts and CSS; synthetic read-only API.
import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { SearchView } from '../features/search/SearchView';
import { Sidebar } from '../app/Sidebar';
import type { ViewId } from '../app/App';
import reviewImage from '../../../docs/features/images/current-workbench.ko.jpg?url';
import '../theme/tokens.css';
import '../theme/legacy-components.css';
import '../theme/refresh.css';
const params=new URLSearchParams(location.search || ({'moc.html':'moc','list.html':'list','error.html':'error','media-error.html':'media-error','empty.html':'empty','loading.html':'loading','en.html':'lang=en','search.html':'search'}[location.pathname.split('/').at(-1)!] || ''));document.documentElement.lang=params.get('lang')||'ko';
const markdown=`# 좋은 Knowledge를 남기는 방법

작업의 결론만큼 중요한 것은, 다음에 같은 문제를 만났을 때 다시 사용할 수 있는 맥락입니다.

> 완료와 발행은 서로 다른 결정입니다. 사용자가 검토한 지식만 Vault에 남깁니다.

## 01. 생각에서 재사용 가능한 지식으로

Capture에 남긴 생각을 Task로 구체화하고, 검증한 결과를 Knowledge로 연결합니다. 원래의 질문과 선택한 이유를 함께 기록하세요.

\`\`\`mermaid
flowchart LR
  Capture --> Task --> Review[사용자 검토] --> Knowledge
  classDef review fill:#fff0f6,stroke:#ffc6df
  classDef knowledge fill:#e9f9f1,stroke:#ded6da
  class Review review
  class Knowledge knowledge
\`\`\`

## 02. 다음 작업에 필요한 것

- **맥락** — 어떤 문제를 해결했는지
- **결정** — 무엇을 선택했고 왜 선택했는지
- **근거** — 검증 결과와 다시 확인할 자료

## 03. 이미지로 맥락 남기기

![Task와 Capture를 함께 보여주는 실제 LLM Wiki Workbench 예시](../../assets/workbench.jpg)

작업 맥락을 함께 보존하는 Workbench · 프로젝트 문서 이미지

## 검토 체크리스트

| 항목 | 확인할 내용 |
| --- | --- |
| 재사용성 | 처음 읽는 사람도 따라 할 수 있는가? |
| 근거 | 실제 검증과 제약이 남아 있는가? |

관련 노트: [[Knowledge/MOCs/Guide|Knowledge MOC로 돌아가기 →]]

<script>window.evil=true</script>
`;
const moc=`# Knowledge 운영 가이드

생각을 남기고, 검토하고, 다시 찾기까지.\u0020\u0020
Knowledge를 오래 쓸 수 있게 만드는 안내서입니다.

## 시작하기

[[Knowledge/Guides/knowledge-writing|좋은 Knowledge를 남기는 방법]]

맥락·결정·근거를 연결하는 기본 원칙

[[Knowledge/Guides/capture|Capture에서 Task로]]

원문을 보존하며 질문을 구체화하기

## 검토와 발행

[[Knowledge/Guides/publish|발행 전에 확인할 것]]

[[Knowledge/Guides/images|이미지와 출처 남기기]]

## 다시 찾기

폴더는 저장 위치를, MOC는 노트 사이의 연결을 보여줍니다. 같은 노트에 두 경로로 접근할 수 있습니다.
`;
const notes=[['Knowledge/Guides/knowledge-writing.md','좋은 Knowledge를 남기는 방법'],['Knowledge/Guides/capture.md','Capture에서 Task로'],['Knowledge/Guides/publish.md','발행 전에 확인할 것'],['Knowledge/Guides/images.md','이미지와 출처 남기기'],['Knowledge/MOCs/Guide.md','Knowledge 운영 가이드'],['Knowledge/MOCs/Product.md','제품 설계'],['Knowledge/MOCs/Engineering.md','Engineering'],['Knowledge/MOCs/Research.md','리서치 노트']].map(([path,title])=>({path,title,isMoc:path.includes('/MOCs/'),managed:true,readable:true,modifiedAt:1792540800}));
window.llmWikiApplication={request:async({path})=>{const u=new URL(path,'http://fixture');let body:unknown={};let ok=true;
if(u.pathname==='/vault/browse')body={notes,folders:['Knowledge','Knowledge/Guides','Knowledge/Engineering','Knowledge/Research','Knowledge/MOCs','Empty']};
if(u.pathname==='/knowledge') {if(params.has('error'))ok=false;if(params.has('loading'))await new Promise(()=>{});body={path:u.searchParams.get('path'),markdown:u.searchParams.get('path')?.includes('/MOCs/')?moc:params.has('media-error')?markdown.replace('flowchart LR','invalid diagram'):markdown,canonical_locale:'original',served_locale:'original',translated:false};}
if(u.pathname==='/vault/image'){if(params.has('media-error'))ok=false;const dataUrl=reviewImage;body={dataUrl};}
if(u.pathname==='/search')body={results:notes.map(note=>({...note,snippet:'검증한 결과를 Knowledge로 연결합니다.'})),has_more:false};
return {ok,status:ok?200:404,json:async<T,>()=>body as T,text:async()=>JSON.stringify(body),body:null};}};
export function VaultReviewFixture(){const [active,setActive]=useState<ViewId>('search');return <div className="app"><Sidebar activeView={active} onSelectView={setActive}/><main><SearchView active={active==='search'}/></main></div>;}
createRoot(document.getElementById('root')!).render(<VaultReviewFixture/>);
setTimeout(()=>{(document.querySelector(`[data-control="vault-tab"]:nth-child(${params.has('moc')?2:params.has('search')?1:3})`) as HTMLButtonElement)?.click();setTimeout(()=>{
if(params.has('search')){const field=document.querySelector<HTMLInputElement>('#query')!;const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!;setter.call(field,'Knowledge');field.dispatchEvent(new Event('input',{bubbles:true}));document.querySelector<HTMLFormElement>('#search-form')!.requestSubmit();setTimeout(()=>document.querySelector<HTMLButtonElement>('[data-control=search-result-open]')?.click(),100);}
else if(!params.has('moc')){(Array.from(document.querySelectorAll('[data-control="vault-folder-select"]')).find(node=>node.textContent?.startsWith('Knowledge')) as HTMLButtonElement)?.click();setTimeout(()=>{(Array.from(document.querySelectorAll('[data-control="vault-folder-select"]')).find(node=>node.textContent?.startsWith('Guides')) as HTMLButtonElement)?.click();setTimeout(()=>{if(params.has('empty'))(Array.from(document.querySelectorAll('[data-control="vault-folder-select"]')).find(node=>node.textContent?.startsWith('Empty')) as HTMLButtonElement)?.click();else {(document.querySelector('[data-control="vault-note-open"]') as HTMLButtonElement)?.click();if(params.has('list'))setTimeout(()=>document.querySelector<HTMLButtonElement>('[data-control="vault-list-back"]')?.click(),100);}},100);},100);}
else (document.querySelector('[data-control="vault-moc-open"]') as HTMLButtonElement)?.click();
},200);},200);
