import { useEffect, useRef, useState } from 'react';
import { IconButton } from '../../components/IconButton';
import { useWorkTrackingText } from '../chat/useWorkTrackingText';

let refreshSequence=0;
const refreshWorkbenchSnapshot = async () => {
  const sequence=++refreshSequence;
  if (!window.llmWikiApplication) return;
  try {
  const response=await window.llmWikiApplication.request({path:'/work-tracking/current'});
  if (!response.ok) return;
  const detail=await response.json();
  if(sequence===refreshSequence) window.dispatchEvent(new CustomEvent('llm-wiki:work-tracking-refresh',{detail}));
  } catch { /* Keep the last snapshot; a later focus/read retries without disturbing input. */ }
};

export function WorkbenchView({ active }: { active: boolean }) {
  const t=useWorkTrackingText();
  const [tracked, setTracked] = useState<Array<{ trackedSessionId: string; capture: string; headRevision: number; sourceInterface: string; state: string; recentProgress?: Array<{eventId:string;payload:{summary?:string}}> }>>([]);
  useEffect(() => {
    const update = (event: Event) => setTracked((event as CustomEvent).detail.activeWork ?? []);
    window.addEventListener('llm-wiki:work-tracking-refresh', update);
    return () => window.removeEventListener('llm-wiki:work-tracking-refresh', update);
  }, []);
  const activeRef=useRef(active);
  activeRef.current=active;
  useEffect(()=>{
    const refresh=()=>{if(activeRef.current) void refreshWorkbenchSnapshot();};
    const visible=()=>{if(document.visibilityState==='visible')refresh();};
    window.addEventListener('focus',refresh);
    document.addEventListener('visibilitychange',visible);
    window.addEventListener('llm-wiki:work-tracking-written',refresh);
    const timer=window.setInterval(refresh,15_000);
    if(active)refresh();
    return()=>{window.removeEventListener('focus',refresh);document.removeEventListener('visibilitychange',visible);window.removeEventListener('llm-wiki:work-tracking-written',refresh);window.clearInterval(timer);};
  },[active]);
  return (
    <section id="workbench" className={`view${active ? ' active' : ''}`}>
      <header className="top">
        <div><div className="eyebrow">Your thinking space</div><h1>What’s on your mind?</h1></div>
        <div className="status">LOCAL • YOUR VAULT</div>
      </header>
      <section className="quick">
        <h2>Start with a Capture.</h2>
        <p>Capture a thought first. Then AI helps turn it into a reviewable Problem.</p>
        <form className="capture" id="capture">
          <input id="capture-text" aria-label="Capture your thought" placeholder="e.g. I keep losing track of decisions…" required />
          <IconButton kind="primary" label="Save Capture" labelVisible>+</IconButton>
        </form>
      </section>
      <div className="board-head">
        <h2>Your workbench</h2>
        <span id="organize-status">Keep attention on what matters now.</span>
        <IconButton id="organize" kind="tiny hot" label="Organize workbench" labelVisible>✦</IconButton>
        <IconButton id="flow-toggle" label="Show flow" labelVisible>⤳</IconButton>
      </div>
      <section id="flow-view" className="flow-view" hidden />
      <section className="board" id="board" />
      <section className="tracked-workbench" aria-label={t('card.region')}>
        {tracked.map((work,index) => <article key={work.trackedSessionId ?? index} className="work-tracking-card">
          <h3 data-user-content>{work.capture}</h3><p>{t('source.'+work.sourceInterface)} · {t('state.'+work.state)} · {t('card.revision')} {work.headRevision}</p>
          {work.recentProgress?.map(entry => <p key={entry.eventId} data-user-content>{entry.payload.summary}</p>)}
          <button type="button" onClick={() => window.dispatchEvent(new CustomEvent('llm-wiki:tracked-resume', {detail:{sessionId:work.trackedSessionId}}))}>{t('workbench.resume')}</button>
        </article>)}
      </section>
      <section className="workbench-context">
        <article className="context-panel">
          <div className="panel-head">
            <div><small>SEARCH VAULT</small><h2>Recently archived</h2></div>
            <IconButton id="archive-more" label="Show more archived documents">⋯</IconButton>
          </div>
          <div id="recent-archive" className="context-list" />
        </article>
        <article className="context-panel">
          <div className="panel-head"><div><small>COMPLETED SOLUTIONS</small><h2>Reusable work</h2></div></div>
          <div id="completed-solutions" className="context-list" />
        </article>
      </section>
    </section>
  );
}
