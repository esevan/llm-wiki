const closeDialog = (id: string) => (document.getElementById(id) as HTMLDialogElement | null)?.close();

function IconAction({ dialog, label }: { dialog: string; label: string }) {
  return <button type="button" className="tiny icon-action" data-tooltip={label} aria-label={label} onClick={() => closeDialog(dialog)}>×</button>;
}

export function OverlayLayer() {
  useEffect(() => {
    const dismissOutside = (event: MouseEvent) => {
      if (!(event.target instanceof Node)) return;
      for (const name of ['queue', 'alert']) {
        const panel = document.getElementById(`${name}-panel`);
        const toggle = document.getElementById(`${name}-toggle`);
        if (!panel || panel.hidden || panel.contains(event.target) || toggle?.contains(event.target)) continue;
        panel.hidden = true;
        toggle?.setAttribute('aria-expanded', 'false');
      }
    };
    // Capture before result actions replace DOM or stop propagation. Never consume the click.
    document.addEventListener('click', dismissOutside, true);
    return () => document.removeEventListener('click', dismissOutside, true);
  }, []);
  return (
    <div className="overlay-layer">
      <dialog id="feature-modal"><form className="modal" id="feature-form"><h2>Propose a way forward</h2><input type="hidden" id="feature-problem" /><label htmlFor="feature-title">Short name</label><input id="feature-title" required placeholder="A clear, human outcome" /><label htmlFor="feature-outcome">Intended outcome</label><textarea id="feature-outcome" required placeholder="What will be true when this works?" /><label htmlFor="feature-nongoals">Non-goals</label><textarea id="feature-nongoals" placeholder="What this will not try to solve" /><footer><IconAction dialog="feature-modal" label="Cancel" /><button className="primary icon-action" data-tooltip="Save proposal" aria-label="Save proposal">✓</button></footer></form></dialog>

      <dialog id="chat-modal"><section className="modal"><h2 id="chat-title">Explore together</h2><div id="chat-workspace"><aside id="explore-refinement-preview" className="explore-refinement-preview" aria-live="polite" hidden><header><small>REFINEMENT PREVIEW</small><div className="preview-head-actions"><span id="explore-preview-status">LIVE CONTEXT</span><button type="button" id="preview-job-status" data-state="idle" data-tooltip="Refinement will update after your next chat response." aria-label="Refinement will update after your next chat response.">•</button></div></header><nav className="preview-tabs" role="tablist" aria-label="Refinement Preview format"><button type="button" id="preview-detail-tab" role="tab" aria-controls="explore-preview-detail" aria-selected="false" disabled>Detail</button><button type="button" id="preview-context-tab" role="tab" aria-controls="explore-preview-content" aria-selected="true">Context</button></nav><div id="explore-preview-detail" role="tabpanel" aria-labelledby="preview-detail-tab" hidden /><div id="explore-preview-content" role="tabpanel" aria-labelledby="preview-context-tab" /></aside><section id="chat-column"><div id="chat-log" className="results" /><form id="chat-form"><label htmlFor="chat-message">What would you like to think through?</label><textarea id="chat-message" required placeholder="What feels unclear, risky, or worth exploring?" /><footer><IconAction dialog="chat-modal" label="Close" /><button className="primary icon-action" data-tooltip="Ask" aria-label="Ask">↑</button></footer></form></section></div></section></dialog>

      <dialog id="draft-modal" aria-busy="false"><form className="modal" id="draft-form"><h2 id="draft-title">Review proposal</h2><p className="meta" id="draft-meta" role="status" aria-live="polite">Review the proposed structure, edit it if needed, then explicitly finalize it.</p><input type="hidden" id="draft-type" /><input type="hidden" id="draft-id" /><input type="hidden" id="draft-mode" /><div className="draft-section" id="draft-main-field"><label id="draft-main-label" htmlFor="draft-main">Title</label><input id="draft-main" required /></div><div className="draft-section" id="draft-detail-field"><label id="draft-detail-label" htmlFor="draft-detail">Details</label><textarea id="draft-detail" /></div><div className="draft-section" id="draft-extra-field"><label id="draft-extra-label" htmlFor="draft-extra">Additional details</label><textarea id="draft-extra" /></div><footer><IconAction dialog="draft-modal" label="Cancel" /><button className="primary icon-action" id="draft-submit" data-tooltip="Finalize proposal" aria-label="Finalize proposal">✓</button></footer></form></dialog>

      <dialog id="manual-modal"><form className="modal" id="manual-form"><h2>Manual update</h2><input type="hidden" id="manual-type" /><input type="hidden" id="manual-id" /><label id="manual-title-label" htmlFor="manual-title">Title</label><input id="manual-title" required /><label id="manual-detail-label" htmlFor="manual-detail">Details</label><textarea id="manual-detail" /><label id="manual-version-option" hidden><input id="manual-localized-only" type="checkbox" /> Save as this language version only</label><footer><IconAction dialog="manual-modal" label="Cancel" /><button className="primary icon-action" data-tooltip="Save manually" aria-label="Save manually">✓</button></footer></form></dialog>

      <div className="alert-dock"><button id="alert-toggle" className="alert-toggle" aria-label="Notifications" aria-expanded="false">◌</button><span id="alert-badge" className="alert-badge" hidden>0</span><section id="alert-panel" className="alert-panel" hidden><header><h2>Notifications</h2><button className="tiny" type="button" aria-label="Close">×</button></header><div id="alert-list" className="queue-list" /></section></div>
      <div className="queue-dock"><section id="queue-panel" className="queue-panel" hidden><header><h2>AI Queue</h2><button className="tiny" type="button" aria-label="Close">×</button></header><div id="queue-list" className="queue-list" aria-live="polite" /></section><button id="queue-toggle" className="queue-toggle" aria-label="AI Queue" aria-expanded="false" data-active="false">◴</button></div>
      <aside id="queue-toast" className="queue-toast" role="status" hidden><button className="tiny" type="button" aria-label="Dismiss notification">×</button><strong id="queue-toast-title" /></aside>

      <dialog id="item-detail-modal"><section className="modal item-detail"><div className="eyebrow" id="item-detail-type" /><h2 id="item-detail-title" /><dl id="item-detail-notes" /><footer><button className="primary icon-action" type="button" id="item-detail-close" data-tooltip="Close details" aria-label="Close details">×</button></footer></section></dialog>
      <dialog id="notice-modal" aria-labelledby="notice-title"><section className="modal notice-modal"><div className="eyebrow" id="notice-kicker">LLM WIKI</div><h2 id="notice-title" /><p id="notice-message" /><footer><button type="button" id="notice-cancel" className="tiny" hidden>Cancel</button><button type="button" id="notice-confirm" className="primary">Okay</button></footer></section></dialog>
      <dialog id="transition-modal" aria-labelledby="transition-title"><form className="modal" id="transition-form"><div className="eyebrow manual-kicker">Manual · AI not used</div><h2 id="transition-title">Manual transition</h2><p className="meta" id="transition-description" /><input type="hidden" id="transition-id" /><input type="hidden" id="transition-entity-type" /><input type="hidden" id="transition-entity-id" /><div id="transition-fields" /><footer className="transition-actions"><IconAction dialog="transition-modal" label="Cancel" /><button className="primary" id="transition-submit">Continue</button></footer></form></dialog>
    </div>
  );
}
import { useEffect } from 'react';
