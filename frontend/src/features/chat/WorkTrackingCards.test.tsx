import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { WorkTrackingCards } from './WorkTrackingCards';

describe('WorkTrackingCards',()=>{
  afterEach(() => { document.documentElement.lang = 'en'; });
  it('reviews a proposed draft without invoking publication', () => {
    const review = vi.fn(), publish = vi.fn();
    render(<WorkTrackingCards cards={[{id:'offer',stage:'knowledge',title:'Completed',summary:'Private',revision:1,publicationState:'offered'}]} onReviewDraft={review} onPublish={publish}/>);
    fireEvent.click(screen.getByRole('button', {name:'Review Knowledge draft'}));
    expect(review).toHaveBeenCalledOnce();
    expect(publish).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', {name:/Publish/})).not.toBeInTheDocument();
  });
  it('localizes decisions and pending status in Korean', () => {
    document.documentElement.lang = 'ko';
    render(<WorkTrackingCards cards={[{id:'p',stage:'problem',title:'제안',summary:'검토',revision:2,projectionStatus:'pending_review'}]}/>);
    for (const name of ['수락', '수정', '거절']) expect(screen.getByRole('button', {name})).toBeVisible();
    expect(screen.getByRole('status')).toHaveTextContent('검토 대기');
  });
  it('keeps milestone decisions explicit and accessible',()=>{
    const accept=vi.fn();
    render(<WorkTrackingCards cards={[{id:'p',stage:'problem',title:'Problem proposal',summary:'Review this exact change',revision:2,projectionStatus:'pending_review'}]} onAccept={accept}/>);
    expect(screen.getByRole('status')).toHaveTextContent('pending review');
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    expect(accept).toHaveBeenCalledOnce();
  });

  it('offers publication separately and allows deferral without completing it',()=>{
    const defer=vi.fn(); const publish=vi.fn();
    render(<WorkTrackingCards cards={[{id:'k',stage:'knowledge',title:'Completed work',summary:'Knowledge is still private',revision:7,publicationState:'offered'}]} onPublish={publish} onDeferPublication={defer}/>);
    fireEvent.click(screen.getByRole('button',{name:'Not now'}));
    expect(defer).toHaveBeenCalledOnce();
    expect(publish).not.toHaveBeenCalled();
  });

  it('shows the exact saved draft before the separate publish action',()=>{
    const publish=vi.fn();
    render(<WorkTrackingCards cards={[{id:'draft',stage:'knowledge',title:'Draft',summary:'Private preview',revision:8,publicationState:'draft_saved',draftMarkdown:'# Reviewed\n\nExact body'}]} onPublish={publish}/>);
    fireEvent.click(screen.getByText('Review Knowledge draft'));
    expect(screen.getByText(/Exact body/)).toBeVisible();
    fireEvent.click(screen.getByRole('button',{name:'Publish exact draft'}));
    expect(publish).toHaveBeenCalledOnce();
  });
});
