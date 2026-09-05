import { useSyncExternalStore } from 'react';
import english from '../../../public/i18n/en.json';
import korean from '../../../public/i18n/ko.json';
import type { WorkTrackingCardModel } from '../../types/application';

interface Props {
  cards: WorkTrackingCardModel[];
  busy?: boolean;
  onAccept?: (card: WorkTrackingCardModel) => void;
  onEdit?: (card: WorkTrackingCardModel) => void;
  onReject?: (card: WorkTrackingCardModel) => void;
  onReviewDraft?: (card: WorkTrackingCardModel) => void;
  onPublish?: (card: WorkTrackingCardModel) => void;
  onDeferPublication?: (card: WorkTrackingCardModel) => void;
}

function subscribeLocale(changed: () => void) {
  const observer = new MutationObserver(changed);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
  return () => observer.disconnect();
}

export function WorkTrackingCards({ cards, busy=false, onAccept, onEdit, onReject, onReviewDraft, onPublish, onDeferPublication }: Props) {
  const locale = useSyncExternalStore<'en' | 'ko'>(subscribeLocale, () => document.documentElement.lang.startsWith('ko') ? 'ko' : 'en', () => 'en');
  const resources: Record<string, string> = locale === 'ko' ? korean : english;
  const translate = (key: string) => resources['work_tracking.card.' + key] ?? key;
  const text = Object.fromEntries(['region', 'revision', 'review', 'defer', 'editDraft', 'publish', 'reject', 'edit', 'accept'].map(key => [key, translate(key)]));
  return <section id="work-tracking-cards" className="work-tracking-cards" aria-label={text.region} aria-live="polite">
    {cards.map(card => <article key={card.id} className="work-tracking-card" data-stage={card.stage} data-projection={card.projectionStatus}>
      <header><small>{translate('stage.' + card.stage)}</small><span>{text.revision} {card.revision}</span></header>
      <h3 data-user-content>{card.title}</h3><p data-user-content>{card.summary}</p>
      {card.projectionStatus && <p role="status">{translate('status.' + card.projectionStatus)}</p>}
      {card.stage === 'knowledge' && card.publicationState === 'draft_saved' && <details><summary>{text.review}</summary><pre data-user-content>{card.draftMarkdown}</pre></details>}
      {card.stage === 'knowledge' && card.publicationState === 'offered'
        ? <footer><button type="button" disabled={busy || !onDeferPublication} onClick={()=>onDeferPublication?.(card)}>{text.defer}</button><button type="button" disabled={busy || !onReviewDraft} onClick={()=>onReviewDraft?.(card)}>{text.review}</button></footer>
        : card.stage === 'knowledge' && card.publicationState === 'draft_saved'
          ? <footer><button type="button" disabled={busy || !onEdit} onClick={()=>onEdit?.(card)}>{text.editDraft}</button><button type="button" disabled={busy || !onPublish || !card.draftMarkdown} onClick={()=>onPublish?.(card)}>{text.publish}</button></footer>
        : (card.stage !== 'knowledge' || card.publicationState === 'draft_review') && <footer><button type="button" disabled={busy || !onReject} onClick={()=>onReject?.(card)}>{text.reject}</button>{card.editable !== false && <button type="button" disabled={busy || !onEdit} onClick={()=>onEdit?.(card)}>{text.edit}</button>}<button type="button" disabled={busy || !onAccept} onClick={()=>onAccept?.(card)}>{text.accept}</button></footer>}
    </article>)}
  </section>;
}
