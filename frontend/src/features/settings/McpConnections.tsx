import { useEffect, useState, type FormEvent } from 'react';
import { useWorkTrackingText } from '../chat/useWorkTrackingText';

interface Connection { id:string; name:string; state:string; scopes:string[]; topicIds:string[] }
const scopes=['session:read','session:write','topic:read','workbench:current:read','workbench:overview:read','vault:search:lexical','vault:search:semantic','vault:evidence:read','knowledge:draft:write','knowledge:publish'];

export function McpConnections() {
  const t=useWorkTrackingText();
  const [connections,setConnections]=useState<Connection[]>([]);
  const [busy,setBusy]=useState(false),[error,setError]=useState(''),[status,setStatus]=useState('');
  const load=async()=>{
    if(!window.llmWikiApplication)return;
    const response=await window.llmWikiApplication.request({path:'/work-tracking/connections'});
    if(!response.ok)throw Error('request_failed');
    setConnections((await response.json<{connections:Connection[]}>()).connections);
  };
  useEffect(()=>{void load().catch(()=>setError('connections.failed'));},[]);
  const run=async(operation:()=>Promise<void>)=>{if(busy)return;setBusy(true);setError('');setStatus('');try{await operation();}catch{setError('connections.failed');}finally{setBusy(false);}};
  const save=async(event:FormEvent<HTMLFormElement>)=>{
    event.preventDefault();const form=event.currentTarget,data=new FormData(form);
    await run(async()=>{
      const response=await window.llmWikiApplication.request({path:'/work-tracking/connections',method:'POST',body:JSON.stringify({name:data.get('name'),scopes:data.getAll('scope'),topicIds:String(data.get('topics')||'').split(',').map(s=>s.trim()).filter(Boolean),checkpointPolicy:'confirm_each'})});
      if(!response.ok)throw Error('request_failed');
      form.reset();await load();
    });
  };
  const membership=async(event:FormEvent<HTMLFormElement>)=>{
    event.preventDefault();const data=new FormData(event.currentTarget);
    await run(async()=>{
      const response=await window.llmWikiApplication.request({path:'/work-tracking/topic-membership',method:'PUT',body:JSON.stringify({topicId:data.get('topic'),entityType:data.get('type'),entityId:data.get('member'),included:data.get('included')==='yes'})});
      if(!response.ok)throw Error('request_failed');
      setStatus('topics.saved');window.dispatchEvent(new Event('llm-wiki:work-tracking-written'));
    });
  };
  return <section className="mcp-connections" aria-labelledby="mcp-connections-title">
    <h2 id="mcp-connections-title">{t('connections.title')}</h2><p>{t('connections.description')}</p>
    <form data-control="mcp-connection-form" onSubmit={save}><label>{t('connections.name')}<input data-control="mcp-connection-name" name="name" required maxLength={80}/></label>
      <fieldset disabled={busy}><legend>{t('connections.grant')}</legend>{scopes.map(scope=><label key={scope}><input type="checkbox" data-control={`mcp-scope-${scope}`} name="scope" value={scope} defaultChecked={!['topic:read','workbench:overview:read','knowledge:publish'].includes(scope)}/>{t('scope.'+scope)}</label>)}</fieldset>
      <label>{t('connections.topics')}<input data-control="mcp-connection-topics" name="topics" maxLength={1200}/></label><button data-control="mcp-connection-create" disabled={busy}>{t('connections.create')}</button>
    </form>
    {error&&<p role="alert">{t(error)}</p>}{status&&<p role="status">{t(status)}</p>}
    <ul>{connections.map(connection=><li key={connection.id}><strong data-user-content>{connection.name}</strong> · {t('connectionState.'+connection.state)}
      <details data-control="mcp-connection-access"><summary>{t('connections.access')}</summary><ul>{connection.scopes.map(scope=><li key={scope}>{t('scope.'+scope)}</li>)}</ul><p data-user-content>{connection.topicIds?.join(', ')}</p><code data-user-content>llm-wiki-desktop --mcp --connection {connection.id}</code></details>
      {connection.state==='active'&&<button data-control="mcp-connection-revoke" type="button" disabled={busy} onClick={()=>void run(async()=>{const response=await window.llmWikiApplication.request({path:'/work-tracking/connections/'+encodeURIComponent(connection.id),method:'DELETE'});if(!response.ok)throw Error('request_failed');await load();})}>{t('connections.revoke')}</button>}
    </li>)}</ul>
    <details data-control="mcp-topic-details"><summary>{t('topics.title')}</summary><p>{t('topics.description')}</p><form data-control="mcp-topic-form" onSubmit={membership}>
      <label>{t('topics.id')}<input data-control="mcp-topic-id" name="topic" required maxLength={120}/></label>
      <label>{t('topics.type')}<select data-control="mcp-topic-type" name="type">{['vault','captures','problems','features'].map(type=><option key={type} value={type}>{t('memberType.'+type)}</option>)}</select></label>
      <label>{t('topics.member')}<input data-control="mcp-topic-member" name="member" required maxLength={1000}/></label>
      <label>{t('topics.operation')}<select data-control="mcp-topic-included" name="included"><option value="yes">{t('topics.include')}</option><option value="no">{t('topics.remove')}</option></select></label>
      <button data-control="mcp-topic-save" disabled={busy}>{t('topics.save')}</button>
    </form></details>
  </section>;
}
