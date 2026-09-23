import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SettingsView } from './SettingsView';

describe('Chat connection scopes',()=>{
  it('uses the selected locale for the API key privacy note', async () => {
    const request = vi.fn().mockResolvedValue({ok:true,status:200,text:async()=>'',json:async()=>({connections:[]})});
    window.llmWikiApplication = { request };
    document.documentElement.lang = 'ko';
    render(<SettingsView active/>);
    expect(await screen.findByText('API 키는 이 기기의 로컬 설정 파일에만 저장됩니다. Vault나 앱 데이터베이스에는 저장되지 않습니다.')).toBeInTheDocument();
    document.documentElement.lang = 'en';
  });

  it('uses the shared Codex home by default and saves an explicit alternate only on request', async () => {
    const request = vi.fn().mockImplementation((input: { path: string; method?: string }) => {
      if (input.path === '/settings/vault') return Promise.resolve({ ok: true, status: 200, json: async () => ({ path: '/vault' }) });
      if (input.path === '/settings/codex-home' && !input.method) return Promise.resolve({ ok: true, status: 200, json: async () => ({ mode: 'default', defaultPath: '/Users/me/.codex', alternatePath: null }) });
      if (input.path === '/settings/codex-home') return Promise.resolve({ ok: true, status: 200, json: async () => ({ mode: 'alternate', defaultPath: '/Users/me/.codex', alternatePath: '/Volumes/team-codex' }) });
      return Promise.resolve({ ok: true, status: 200, text: async () => '', json: async () => ({ connections: [] }) });
    });
    window.llmWikiApplication = { request };
    render(<SettingsView active />);
    expect(await screen.findByText('/Users/me/.codex')).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText('Use a different Codex home for LLM Wiki'));
    fireEvent.change(screen.getByLabelText('Alternate Codex home'), { target: { value: '/Volumes/team-codex' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save Codex home' }));
    await waitFor(() => expect(request).toHaveBeenCalledWith({ path: '/settings/codex-home', method: 'PUT', body: JSON.stringify({ alternatePath: '/Volumes/team-codex' }) }));
    expect(await screen.findByText('Saved. Restart LLM Wiki before opening or running a Codex conversation.')).toBeInTheDocument();
  });

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

  it('revokes an active connection and saves an explicit topic membership', async () => {
    const request=vi.fn().mockResolvedValue({ok:true,status:200,text:async()=>'',json:async()=>({connections:[{id:'active connection',name:'Scoped Chat',state:'active',scopes:['session:read'],topicIds:['LLM Wiki']}]})});
    window.llmWikiApplication={request};
    render(<SettingsView active/>);

    await screen.findByRole('button',{name:'Revoke'});
    fireEvent.click(screen.getByRole('button',{name:'Revoke'}));
    await waitFor(()=>expect(request).toHaveBeenCalledWith({path:'/work-tracking/connections/active%20connection',method:'DELETE'}));

    fireEvent.click(screen.getByText('Manage topic membership'));
    fireEvent.change(screen.getByLabelText('Topic ID'),{target:{value:'release'}});
    fireEvent.change(screen.getByLabelText('Member type'),{target:{value:'features'}});
    fireEvent.change(screen.getByLabelText('Item ID or indexed Vault relative path'),{target:{value:'feature-42'}});
    fireEvent.change(screen.getByLabelText('Membership change'),{target:{value:'no'}});
    fireEvent.click(screen.getByRole('button',{name:'Apply membership'}));

    await waitFor(()=>expect(request).toHaveBeenCalledWith({
      path:'/work-tracking/topic-membership',method:'PUT',
      body:JSON.stringify({topicId:'release',entityType:'features',entityId:'feature-42',included:false}),
    }));
    expect(screen.getByText('Topic membership updated.')).toHaveAttribute('role','status');
  });
});
