import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WorkbenchView } from './WorkbenchView';

describe('Workbench refresh',()=>{
  it('refreshes on focus without replacing the user draft, caret, or focus',async()=>{
    window.llmWikiApplication={request:vi.fn().mockResolvedValue({ok:true,status:200,json:async()=>({workspaceRevision:2}),text:async()=>'',body:null})};
    render(<WorkbenchView active/>);
    const input=screen.getByPlaceholderText('e.g. I keep losing track of decisions…') as HTMLInputElement;
    const scroller=document.createElement('div');scroller.id='preserved-scroll';scroller.scrollTop=37;document.body.appendChild(scroller);
    const disclosure=document.createElement('details');disclosure.open=true;document.body.appendChild(disclosure);
    const stream=document.createElement('div');stream.textContent='streaming response';document.body.appendChild(stream);
    input.focus();fireEvent.compositionStart(input);fireEvent.input(input,{target:{value:'작성 중인 캡처'}});input.setSelectionRange(3,5);
    window.dispatchEvent(new Event('focus'));
    await waitFor(()=>expect(window.llmWikiApplication.request).toHaveBeenCalled());
    expect(input).toHaveValue('작성 중인 캡처');
    expect(input.selectionStart).toBe(3);
    expect(input.selectionEnd).toBe(5);
    expect(document.activeElement).toBe(input);
    expect(scroller.scrollTop).toBe(37);
    expect(disclosure.open).toBe(true);
    expect(stream).toHaveTextContent('streaming response');
    fireEvent.compositionEnd(input);
  });
});
