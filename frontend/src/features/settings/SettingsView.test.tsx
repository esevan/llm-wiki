import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SettingsView } from './SettingsView';

describe('Chat connection scopes',()=>{
  it('keeps whole-Workbench, topic, and publication grants explicit',async()=>{
    const request=vi.fn().mockResolvedValue({ok:true,status:200,text:async()=>'',json:async()=>({connections:[]})});
    window.llmWikiApplication={request};
    render(<SettingsView active/>);
    await waitFor(()=>expect(request).toHaveBeenCalledWith({path:'/work-tracking/connections'}));
    fireEvent.change(screen.getByLabelText('Connection name'),{target:{value:'Scoped Chat'}});
    fireEvent.click(screen.getByLabelText('Read explicitly selected topics'));
    fireEvent.click(screen.getByLabelText('Read the whole Workbench overview'));
    fireEvent.click(screen.getByLabelText('Publish exact reviewed drafts'));
    fireEvent.change(screen.getByLabelText(/Allowed topic IDs/),{target:{value:'LLM Wiki, Release'}});
    fireEvent.click(screen.getByRole('button',{name:'Create connection'}));
    await waitFor(()=>expect(request).toHaveBeenCalledWith(expect.objectContaining({
      path:'/work-tracking/connections',method:'POST',
      body:expect.stringContaining('workbench:overview:read'),
    })));
    const createCall=request.mock.calls.find(call=>call[0].method==='POST');
    expect(createCall).toBeDefined();
    const create=JSON.parse(createCall![0].body);
    expect(create.topicIds).toEqual(['LLM Wiki','Release']);
    expect(create.scopes).toContain('knowledge:publish');
  });
});
