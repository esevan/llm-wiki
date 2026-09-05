import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ChatTrackingSurface } from './ChatTrackingSurface';

describe('ChatTrackingSurface actions', () => {
  afterEach(() => { document.documentElement.lang = 'en'; });

  it('dispatches proposal, decision, publication, and edited-payload actions only after their buttons are clicked', () => {
    const actions = vi.fn();
    const proposals = vi.fn();
    window.addEventListener('llm-wiki:chat-tracking-action', actions as EventListener);
    window.addEventListener('llm-wiki:chat-tracking-propose', proposals as EventListener);
    render(<ChatTrackingSurface />);
    act(() => window.dispatchEvent(new CustomEvent('llm-wiki:chat-tracking', { detail: {
      actions: ['adopt_problem'],
      cards: [
        {id:'proposal',stage:'problem',title:'Problem',summary:'Review',revision:1,payload:{title:'Problem'}},
        {id:'offer',stage:'knowledge',title:'Offer',summary:'Private',revision:2,publicationState:'offered'},
        {id:'draft',stage:'knowledge',title:'Draft',summary:'Saved',revision:3,publicationState:'draft_saved',draftMarkdown:'# Draft'},
      ],
    } })));

    fireEvent.click(screen.getByRole('button', { name: 'Review Problem proposal' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
    fireEvent.click(screen.getByRole('button', { name: 'Accept' }));
    fireEvent.click(screen.getByRole('button', { name: 'Not now' }));
    fireEvent.click(screen.getByRole('button', { name: 'Review Knowledge draft' }));
    fireEvent.click(screen.getByRole('button', { name: 'Publish exact draft' }));
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    expect(screen.getByRole('textbox', { name: 'Edit review content' })).toHaveValue('{\n  "title": "Problem"\n}');
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('textbox', { name: 'Edit review content' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Edit review content' }), { target: { value: '{"title":"Revised"}' } });
    fireEvent.click(screen.getByRole('button', { name: 'New preview' }));

    expect(proposals).toHaveBeenCalledTimes(1);
    expect((proposals.mock.calls[0][0] as CustomEvent).detail).toEqual({action:'adopt_problem'});
    expect(actions.mock.calls.map(([event]) => (event as CustomEvent).detail)).toEqual([
      {action:'reject',id:'proposal',payload:undefined},
      {action:'accept',id:'proposal',payload:undefined},
      {action:'defer',id:'offer',payload:undefined},
      {action:'review-draft',id:'offer',payload:undefined},
      {action:'publish',id:'draft',payload:undefined},
      {action:'edit',id:'proposal',payload:{title:'Revised'}},
    ]);
    window.removeEventListener('llm-wiki:chat-tracking-action', actions as EventListener);
    window.removeEventListener('llm-wiki:chat-tracking-propose', proposals as EventListener);
  });
});
