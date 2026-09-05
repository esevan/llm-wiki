import { useEffect, useState, useSyncExternalStore } from 'react';
import type { WorkTrackingCardModel } from '../../types/application';
import { WorkTrackingCards } from './WorkTrackingCards';
import english from '../../../public/i18n/en.json';
import korean from '../../../public/i18n/ko.json';

type EditableCard = WorkTrackingCardModel & { payload?: Record<string, unknown> };
type Snapshot = { cards: EditableCard[]; busy?: boolean; error?: string; actions?: string[] };

function subscribeLocale(changed: () => void) {
  const observer = new MutationObserver(changed);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
  return () => observer.disconnect();
}

export function ChatTrackingSurface() {
  const [snapshot, setSnapshot] = useState<Snapshot>({ cards: [] });
  const [editing, setEditing] = useState<EditableCard | null>(null);
  const [text, setText] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    const update = (event: Event) => setSnapshot((event as CustomEvent<Snapshot>).detail);
    window.addEventListener('llm-wiki:chat-tracking', update);
    window.dispatchEvent(new Event('llm-wiki:chat-tracking-ready'));
    return () => window.removeEventListener('llm-wiki:chat-tracking', update);
  }, []);
  const act = (action: string, card: EditableCard, payload?: unknown) => {
    window.dispatchEvent(new CustomEvent('llm-wiki:chat-tracking-action', { detail: { action, id: card.id, payload } }));
  };
  const ko = useSyncExternalStore(subscribeLocale, () => document.documentElement.lang.startsWith('ko'), () => false);
  const resources:Record<string,string>=ko?korean:english;
  const t=(key:string)=>resources['work_tracking.'+key]??key;
  return <>
    {snapshot.actions?.length ? <nav aria-label={t('chat.actions')}>{snapshot.actions.map(action => <button type="button" key={action} disabled={snapshot.busy} onClick={() => window.dispatchEvent(new CustomEvent('llm-wiki:chat-tracking-propose',{detail:{action}}))}>{t('action.'+action)}</button>)}</nav> : null}
    <WorkTrackingCards cards={snapshot.cards} busy={snapshot.busy}
      onAccept={card => act('accept', card)} onReject={card => act('reject', card)}
      onEdit={card => { setEditing(card); setText(JSON.stringify((card as EditableCard).payload ?? {}, null, 2)); setError(''); }}
      onReviewDraft={card => act('review-draft', card)} onPublish={card => act('publish', card)}
      onDeferPublication={card => act('defer', card)} />
    {snapshot.error && <p role="alert" data-user-content>{snapshot.error}</p>}
    {editing && <form onSubmit={event => { event.preventDefault(); try { const payload: unknown = JSON.parse(text); if (!payload || typeof payload !== 'object' || Array.isArray(payload)) throw new Error(t('chat.invalidObject')); act('edit', editing, payload); setEditing(null); } catch (caught) { setError(String(caught)); } }}>
      <label>{t('chat.edit')}<textarea value={text} onChange={event => setText(event.target.value)} required data-user-content /></label>
      {error && <p role="alert">{error}</p>}
      <button type="button" onClick={() => setEditing(null)}>{t('chat.cancel')}</button>
      <button type="submit">{t('chat.preview')}</button>
    </form>}
  </>;
}
