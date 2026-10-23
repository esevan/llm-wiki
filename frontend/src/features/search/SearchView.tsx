import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { VaultMarkdown } from './VaultMarkdown';
import { noteLinkTargets, resolveNoteLink, vaultClient, type VaultDocument, type VaultListing, type VaultResult } from './vaultClient';
import { useVaultText } from './vaultText';
import './vault-search.css';

type Tab = 'search' | 'moc' | 'folders';
export function SearchView({ active }: { active: boolean }) {
  const text=useVaultText();
  const [tab,setTab]=useState<Tab>('search'); const [filter,setFilter]=useState('');
  const [listing,setListing]=useState<VaultListing>(); const [browseError,setBrowseError]=useState(false); const [browseBusy,setBrowseBusy]=useState(false);
  const [folder,setFolder]=useState(''); const [expanded,setExpanded]=useState<Set<string>>(new Set(['']));
  const [selected,setSelected]=useState(''); const [document,setDocument]=useState<VaultDocument>(); const [loading,setLoading]=useState(false); const [error,setError]=useState(false); const [reload,setReload]=useState(0);
  const [raw,setRaw]=useState(false); const [mobileReader,setMobileReader]=useState(false); const [history,setHistory]=useState<string[]>([]);
  const [query,setQuery]=useState(''); const [semantic,setSemantic]=useState(true); const [results,setResults]=useState<VaultResult[]>([]); const [searchState,setSearchState]=useState<'start'|'loading'|'ready'|'error'>('start'); const [more,setMore]=useState(false); const [moreBusy,setMoreBusy]=useState(false);
  const searchGeneration=useRef(0); const submitted=useRef({query:'',semantic:true});
  const reader=useRef<HTMLElement>(null); const list=useRef<HTMLDivElement>(null); const article=useRef<HTMLElement>(null); const readerScroll=useRef<Record<string,number>>({}); const listScroll=useRef<Partial<Record<Tab,number>>>({}); const heading=useRef(''); const restoring=useRef(false);
  const notes=listing?.notes??[]; const current=notes.find(note=>note.path===selected);
  const loadListing=async()=>{setBrowseBusy(true);setBrowseError(false);try{setListing(await vaultClient.browse());}catch{setBrowseError(true);}finally{setBrowseBusy(false);}};
  useEffect(()=>{if(active&&!listing&&!browseBusy&&!browseError)void loadListing();},[active,listing,browseBusy,browseError]);
  useEffect(()=>{
    if(!selected)return;
    let cancelled=false;setLoading(true);setError(false);setDocument(undefined);
    void vaultClient.read(selected).then(value=>{if(!cancelled)setDocument(value);}).catch(()=>{if(!cancelled)setError(true);}).finally(()=>{if(!cancelled)setLoading(false);});
    return()=>{cancelled=true;};
  },[selected,reload]);
  useEffect(()=>{
    if(!document||!reader.current)return;
    const saved=readerScroll.current[selected]??0;restoring.current=true;
    reader.current.scrollTop=saved;
    article.current?.querySelector<HTMLElement>('h1')?.focus({preventScroll:true});
    const observer=typeof ResizeObserver==='undefined'?null:new ResizeObserver(()=>{if(restoring.current&&reader.current&&!heading.current)reader.current.scrollTop=saved;});
    if(article.current)observer?.observe(article.current);
    const timer=setTimeout(()=>{restoring.current=false;observer?.disconnect();},1500);
    if(heading.current){const target=Array.from(article.current?.querySelectorAll('h1,h2,h3,h4,h5,h6')??[]).find(node=>node.textContent?.trim().toLowerCase()===heading.current.toLowerCase()||node.textContent?.trim().toLowerCase().replace(/\s+/g,'-')===heading.current.toLowerCase());target?.scrollIntoView({block:'start'});heading.current='';}
    return()=>{clearTimeout(timer);observer?.disconnect();};
  },[document,selected]);
  const navigate=(path:string,section='')=>{
    if(reader.current&&selected)readerScroll.current[selected]=reader.current.scrollTop;
    restoring.current=true;
    if(path!==selected){if(selected)setHistory(value=>[...value,selected]);setSelected(path);setRaw(false);}
    heading.current=section;setMobileReader(true);
    if(path===selected)restoring.current=false;
    if(path===selected&&section){const target=Array.from(article.current?.querySelectorAll('h1,h2,h3,h4,h5,h6')??[]).find(node=>node.textContent?.trim().toLowerCase()===section.toLowerCase());target?.scrollIntoView({block:'start'});}
    const parents=path.split('/').slice(0,-1);setExpanded(value=>new Set([...value,...parents.map((_,i)=>parents.slice(0,i+1).join('/'))]));
  };
  const changeTab=(next:Tab)=>{if(list.current)listScroll.current[tab]=list.current.scrollTop;setTab(next);requestAnimationFrame(()=>{if(list.current)list.current.scrollTop=listScroll.current[next]??0;});};
  const tabKeys=(event:KeyboardEvent<HTMLButtonElement>,value:Tab)=>{const tabs:Tab[]=['search','moc','folders'];let index=tabs.indexOf(value);if(event.key==='ArrowRight')index=(index+1)%3;else if(event.key==='ArrowLeft')index=(index+2)%3;else if(event.key==='Home')index=0;else if(event.key==='End')index=2;else return;event.preventDefault();changeTab(tabs[index]);globalThis.document.getElementById('vault-tab-'+tabs[index])?.focus();};
  const search=async(append=false)=>{
    if(append&&moreBusy)return;
    const generation=append?searchGeneration.current:++searchGeneration.current;
    if(!append){submitted.current={query,semantic};setSearchState('loading');setMoreBusy(false);}else setMoreBusy(true);
    const offset=append?results.length:0;
    try{const response=await vaultClient.search(submitted.current.query,submitted.current.semantic,offset);if(generation!==searchGeneration.current)return;setResults(value=>append?[...value,...response.results]:response.results);setMore(response.has_more??response.results.length===20);setSearchState('ready');}
    catch{if(generation===searchGeneration.current)setSearchState('error');}
    finally{if(generation===searchGeneration.current)setMoreBusy(false);}
  };
  const filterValue=filter.trim().toLowerCase();
  const visibleNotes=notes.filter(note=>!filterValue||(note.title+' '+note.path).toLowerCase().includes(filterValue));
  const renderFolder=(path:string,depth=0):React.ReactNode=>{
    const children=(listing?.folders??[]).filter(child=>child.split('/').slice(0,-1).join('/')===path&&(!filterValue||child.toLowerCase().includes(filterValue)||visibleNotes.some(note=>note.path.startsWith(child+'/'))));
    const childrenNotes=visibleNotes.filter(note=>note.path.split('/').slice(0,-1).join('/')===path);
    const isOpen=expanded.has(path)||!!filterValue;
    const name=path.split('/').at(-1)||text.vault;
    return <div key={path} className="vault-folder" style={{'--folder-depth':depth} as React.CSSProperties}>
      {path&&<div className={'vault-folder-row'+(folder===path?' is-folder-selected':'')}>
        <button data-control="vault-folder-disclose" type="button" className="vault-disclosure" aria-label={name} aria-expanded={isOpen} onClick={()=>setExpanded(value=>{const next=new Set(value);if(next.has(path))next.delete(path);else next.add(path);return next;})}>{isOpen?'⌄':'›'}</button>
        <button data-control="vault-folder-select" type="button" className="vault-folder-select" aria-pressed={folder===path} onClick={()=>{setFolder(path);if(!notes.some(note=>!path||note.path.startsWith(path+'/'))){setSelected('');setMobileReader(false);}setExpanded(value=>new Set([...value,path]));}}>{name}<span>{notes.filter(note=>!path||note.path.startsWith(path+'/')).length}</span></button>
      </div>}
      {isOpen&&<div>{children.map(child=>renderFolder(child,depth+1))}{childrenNotes.map(note=><button data-control="vault-note-open" type="button" key={note.path} className={'vault-note-row'+(selected===note.path?' is-selected':'')} aria-current={selected===note.path?'page':undefined} onClick={()=>navigate(note.path)}><span aria-hidden="true">▤</span><span>{note.title}<small>{note.path.split('/').at(-1)}</small></span></button>)}{!children.length&&!childrenNotes.length&&<p className="vault-folder-empty">{filterValue?text.noResults:text.emptyFolder}</p>}</div>}
    </div>;
  };
  const back=()=>{if(reader.current&&selected)readerScroll.current[selected]=reader.current.scrollTop;setMobileReader(false);};
  const mocLinks=document?noteLinkTargets(document.markdown).map(target=>resolveNoteLink(selected,target,notes)).filter(Boolean):[];
  return <section id="search" className={`view vault-search${active?' active':''}${mobileReader?' is-reading':''}`}>
    <aside className="vault-browser" aria-label={text.title}>
      <header><div className="vault-eyebrow">{text.eyebrow}</div><h1 id="search-heading">{text.title}</h1></header>
      <div className="vault-tabs" role="tablist" aria-label={text.title}>{(['search','moc','folders'] as Tab[]).map(value=><button data-control="vault-tab" type="button" role="tab" aria-selected={tab===value} aria-controls="vault-list" id={'vault-tab-'+value} tabIndex={tab===value?0:-1} key={value} onClick={()=>changeTab(value)} onKeyDown={event=>tabKeys(event,value)}>{text[value]}</button>)}</div>
      <form id="search-form" data-react-vault="true" className={tab==='search'?'vault-query':'vault-query is-hidden'} onSubmit={event=>{event.preventDefault();void search();}}><input id="query" data-control="search-query" aria-labelledby="search-heading" data-i18n-placeholder="search.placeholder" placeholder={text.query} value={query} onChange={event=>setQuery(event.target.value)}/><div><label><input id="semantic" data-control="search-semantic" type="checkbox" checked={semantic} onChange={event=>setSemantic(event.target.checked)}/>{text.semantic}</label><button type="submit" data-control="search-submit" aria-label={text.submit}>⌕ {text.search}</button></div></form>
      {tab!=='search'&&<input data-control="vault-filter" className="vault-filter" aria-label={text.filterLabel} placeholder={text.filter} value={filter} onChange={event=>setFilter(event.target.value)}/>}
      <div className="vault-browser-meta"><span>{tab==='moc'?'MOC':text.vault}</span><span>{tab==='moc'?notes.filter(note=>note.isMoc).length:notes.length} {text.notes} <button data-control="vault-refresh" type="button" aria-label={text.refresh} disabled={browseBusy} onClick={()=>void loadListing()}>↻</button></span></div>
      {browseError&&<div className="vault-list-error" role="status"><span>{listing?text.refreshError:text.browseError}</span><button data-control="vault-browse-retry" onClick={()=>void loadListing()}>{text.retry}</button></div>}
      <div className="vault-list" ref={list} id="vault-list" role="tabpanel" aria-labelledby={'vault-tab-'+tab}>
        {tab==='folders'&&(listing?renderFolder('',-1):browseBusy?<p role="status">{text.loading}</p>:null)}
        {tab==='moc'&&<>{visibleNotes.filter(note=>note.isMoc).map(note=><button data-control="vault-moc-open" type="button" className={'vault-moc-row'+(selected===note.path?' is-selected':'')} key={note.path} aria-current={selected===note.path?'page':undefined} onClick={()=>navigate(note.path)}><strong>{note.title}</strong><small>{note.path}</small></button>)}{!notes.some(note=>note.isMoc)&&listing&&<div className="vault-list-state"><strong>{text.noMoc}</strong><p>{text.mocBody}</p></div>}</>}
        <section id="results" className={tab==='search'?'vault-results':'vault-results is-hidden'} aria-live="polite">
          {searchState==='start'&&<div className="vault-list-state"><strong>{text.start}</strong><p>{text.searchBody}</p></div>}
          {searchState==='loading'&&<p role="status">{text.searching}</p>}
          {searchState==='error'&&<div className="vault-list-state" role="status"><strong>{text.searchError}</strong><button data-control="vault-search-retry" onClick={()=>void search()}>{text.retry}</button></div>}
          {(searchState==='ready'||searchState==='error')&&results.map((result,index)=><button aria-current={selected===result.path?'page':undefined} data-control="search-result-open" data-knowledge-path={encodeURIComponent(result.path)} type="button" className={'vault-result-row'+(selected===result.path?' is-selected':'')} key={result.path+index} onClick={()=>navigate(result.path)}><small>{result.path}</small><strong>{result.title}</strong><span>{(result.snippet||'').replace(/<[^>]*>/g,'')}</span></button>)}
          {searchState==='ready'&&!results.length&&<div className="vault-list-state"><strong>{text.noResults}</strong><p>{text.noResultsBody}</p></div>}
          {more&&searchState==='ready'&&<button data-control="search-more" id="more-results" disabled={moreBusy} aria-busy={moreBusy} onClick={()=>void search(true)}>{text.more}</button>}
        </section>
      </div>
    </aside>
    <section aria-label={text.title} className="vault-reader" ref={reader} aria-busy={loading} onWheel={()=>{restoring.current=false;}} onTouchStart={()=>{restoring.current=false;}} onKeyDown={()=>{restoring.current=false;}} onScroll={()=>{if(reader.current&&selected&&!restoring.current)readerScroll.current[selected]=reader.current.scrollTop;}}>
      <nav className="vault-breadcrumb" aria-label="Breadcrumb"><button data-control="vault-list-back" className="vault-back" onClick={back}>{text.back}</button><span data-user-content>{selected?selected.split('/').join(' / '):text.vault}</span>{selected&&<button data-control="vault-source" className="vault-source" disabled={!document} onClick={()=>setRaw(value=>!value)}>{raw?text.rendered:text.source}</button>}</nav>
      <article className="vault-note" ref={article}>
        {loading?<div className="vault-reader-state" role="status"><h2>{text.loading}</h2><p>{text.loadingBody}</p><button data-control="vault-read-cancel" onClick={()=>{setSelected('');setLoading(false);}}>{text.cancel}</button></div>:error?<div className="vault-reader-state" role="alert"><div className="vault-eyebrow">ERROR</div><h2>{text.error}</h2><p>{text.errorBody}</p><button data-control="search-knowledge-retry" data-retry-knowledge={encodeURIComponent(selected)} onClick={()=>setReload(value=>value+1)}>{text.retry}</button></div>:document?<>
          <div className="vault-eyebrow">{current?.isMoc?text.map:text.knowledge}</div>
          <h1 data-user-content tabIndex={-1}>{current?.title||selected.split('/').at(-1)?.replace(/\.md$/,'')}</h1>
          <div className="vault-note-meta"><span className="vault-status">{current?.isMoc?'MOC':current?.managed?text.published:text.original}</span>{current?.modifiedAt&&<span>{new Intl.DateTimeFormat(globalThis.document.documentElement.lang,{year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date(current.modifiedAt*1000))} {text.updated}</span>}{current?.isMoc&&<span>{new Set(mocLinks.map(link=>link!.path)).size} {text.linkCount}</span>}{document.canonical_locale==='en'&&globalThis.document.documentElement.lang.startsWith('ko')&&<span>{text.originalNotice}</span>}</div>
          {history.length>0&&<button data-control="vault-history-back" className="vault-history" onClick={()=>{const previous=history.at(-1)!;setHistory(value=>value.slice(0,-1));if(reader.current)readerScroll.current[selected]=reader.current.scrollTop;setSelected(previous);setRaw(false);}}>{text.backHistory}</button>}
          {raw?<><p>{text.sourceNotice}</p><pre className="vault-raw" data-user-content>{document.markdown}</pre></>:<VaultMarkdown key={selected} markdown={document.markdown.replace(/^(---\r?\n[\s\S]*?\r?\n---\r?\n)?\s*# [^\n]+\n/, '$1')} path={selected} isMoc={current?.isMoc} notes={notes} onNavigate={navigate} text={text}/>}
        </>:<div className="vault-reader-state"><h2>{folder&&!notes.some(note=>note.path.startsWith(folder+'/'))?text.empty:text.choose}</h2><p>{folder&&!notes.some(note=>note.path.startsWith(folder+'/'))?text.emptyBody:text.chooseBody}</p></div>}
      </article>
    </section>
  </section>;
}
