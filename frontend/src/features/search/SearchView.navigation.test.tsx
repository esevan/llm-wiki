import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { SearchView } from './SearchView';
import { vaultClient, type VaultNote } from './vaultClient';
vi.mock('./VaultMarkdown',()=>({VaultMarkdown:({markdown,onNavigate}:{markdown:string;onNavigate:(path:string)=>void})=><div data-testid="rendered-note">{markdown}<button onClick={()=>onNavigate('MOCs/Map.md')}>Follow MOC</button></div>}));
vi.mock('./vaultClient',async importOriginal=>({...await importOriginal<typeof import('./vaultClient')>(),vaultClient:{browse:vi.fn(),read:vi.fn(),search:vi.fn()}}));
const notes:VaultNote[]=[{path:'Guides/Guide.md',title:'Guide',isMoc:false,modifiedAt:1,managed:true,readable:true},{path:'MOCs/Map.md',title:'Map',isMoc:true,modifiedAt:1,managed:false,readable:true}];
beforeEach(()=>{document.documentElement.lang='en';vi.mocked(vaultClient.browse).mockReset().mockResolvedValue({notes,folders:['Guides','MOCs','Empty']});vi.mocked(vaultClient.read).mockReset().mockImplementation(async path=>({path,markdown:'# '+path+'\n\nBody with **Markdown**',canonical_locale:'original',served_locale:'original',translated:false}));vi.mocked(vaultClient.search).mockReset();});
it('navigates actual folder notes and MOCs in the same reader; raw source is read only',async()=>{
 render(<SearchView active/>);await waitFor(()=>expect(vaultClient.browse).toHaveBeenCalled());
 fireEvent.click(screen.getByRole('tab',{name:'Folders'}));
 fireEvent.click(screen.getByRole('button',{name:'Guides 1'}));
 fireEvent.click(screen.getByRole('button',{name:'Guide Guide.md'}));
 await screen.findByTestId('rendered-note');expect(vaultClient.read).toHaveBeenCalledWith('Guides/Guide.md');
 fireEvent.click(screen.getByRole('button',{name:'Source ↗'}));expect(document.querySelector('.vault-raw')).toHaveTextContent('Body with **Markdown**');
 fireEvent.click(screen.getByRole('button',{name:'Reading view'}));fireEvent.click(screen.getByText('Follow MOC'));
 await waitFor(()=>expect(vaultClient.read).toHaveBeenCalledWith('MOCs/Map.md'));
 fireEvent.click(screen.getByRole('button',{name:'← Previous note'}));await waitFor(()=>expect(document.querySelector('.vault-breadcrumb')).toHaveTextContent('Guides / Guide.md'));
 fireEvent.click(screen.getByRole('tab',{name:'MOC'}));expect(screen.getByRole('button',{name:'Map MOCs/Map.md'})).toBeInTheDocument();
});
it('ignores stale note reads and preserves the exact failed selection for retry',async()=>{
 let resolve!:(value:Awaited<ReturnType<typeof vaultClient.read>>)=>void;
 vi.mocked(vaultClient.read).mockImplementationOnce(()=>new Promise(done=>{resolve=done;})).mockRejectedValueOnce(new Error('missing'));
 render(<SearchView active/>);fireEvent.click(screen.getByRole('tab',{name:'Folders'}));await screen.findByRole('button',{name:'Guides 1'});
 fireEvent.click(screen.getByRole('button',{name:'Guides 1'}));fireEvent.click(screen.getByRole('button',{name:'Guide Guide.md'}));
 fireEvent.click(screen.getByRole('tab',{name:'MOC'}));fireEvent.click(screen.getByRole('button',{name:'Map MOCs/Map.md'}));
 await screen.findByRole('alert');expect(screen.getByRole('alert')).toHaveTextContent('10 MiB');await act(async()=>resolve({path:'Guides/Guide.md',markdown:'STALE BODY',canonical_locale:'original',served_locale:'original',translated:false}));
 expect(screen.queryByText('STALE BODY')).not.toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:'Try again'}));await screen.findByTestId('rendered-note');expect(vaultClient.read).toHaveBeenLastCalledWith('MOCs/Map.md');
});
it('retains semantic search and pagination and ignores earlier search responses',async()=>{
 const first=Array.from({length:20},(_,i)=>({path:`${i}.md`,title:`Result ${i}`}));
 vi.mocked(vaultClient.search).mockResolvedValueOnce({results:first,has_more:true}).mockResolvedValueOnce({results:[{path:'extra.md',title:'More'}],has_more:false});
 render(<SearchView active/>);fireEvent.change(screen.getByRole('textbox',{name:'Search vault'}),{target:{value:'task evidence'}});fireEvent.click(document.querySelector('[data-control="search-submit"]')!);
 await screen.findByRole('button',{name:'0.md Result 0'});expect(vaultClient.search).toHaveBeenCalledWith('task evidence',true,0);
 fireEvent.click(screen.getByText('Show 20 more'));await screen.findByText('More');expect(vaultClient.search).toHaveBeenLastCalledWith('task evidence',true,20);
});
it('exposes keyboard tabs, empty folders, filtering, and a retryable listing error',async()=>{
 vi.mocked(vaultClient.browse).mockRejectedValueOnce(new Error('offline'));
 render(<SearchView active/>);await screen.findByText('Could not load the Vault');fireEvent.click(screen.getByRole('button',{name:'Try again'}));await waitFor(()=>expect(vaultClient.browse).toHaveBeenCalledTimes(2));
 const tab=screen.getByRole('tab',{name:'Search'});fireEvent.keyDown(tab,{key:'End'});expect(screen.getByRole('tab',{name:'Folders'})).toHaveAttribute('aria-selected','true');
 await screen.findByRole('button',{name:'Empty 0'});fireEvent.click(screen.getByRole('button',{name:'Empty 0'}));expect(screen.getByText('Empty folder')).toBeVisible();
 fireEvent.change(screen.getByRole('textbox',{name:'Filter notes'}),{target:{value:'Guide'}});expect(screen.queryByRole('button',{name:'Map Map.md'})).not.toBeInTheDocument();
});

it('keeps every tab selection aligned with its displayed content and preserves the highlighted note',async()=>{
 vi.mocked(vaultClient.search).mockResolvedValue({results:[{path:'Guides/Guide.md',title:'Guide'}],has_more:false});
 render(<SearchView active/>);await waitFor(()=>expect(vaultClient.browse).toHaveBeenCalled());
 const assertTab=(name:string)=>{expect(screen.getAllByRole('tab').filter(tab=>tab.getAttribute('aria-selected')==='true')).toEqual([screen.getByRole('tab',{name})]);};
 assertTab('Search');expect(document.querySelector('.vault-query')).not.toHaveClass('is-hidden');
 fireEvent.click(screen.getByRole('tab',{name:'Folders'}));assertTab('Folders');await screen.findByRole('button',{name:'Guides 1'});fireEvent.click(screen.getByRole('button',{name:'Guides 1'}));fireEvent.click(screen.getByRole('button',{name:'Guide Guide.md'}));await screen.findByTestId('rendered-note');
 const selected=document.querySelector('[data-control="vault-note-open"][aria-current="page"]');expect(selected).toHaveClass('is-selected');
 fireEvent.click(screen.getByRole('tab',{name:'MOC'}));assertTab('MOC');expect(screen.getByRole('button',{name:'Map MOCs/Map.md'})).toBeInTheDocument();expect(document.querySelector('.vault-folder')).toBeNull();
 fireEvent.click(screen.getByRole('button',{name:'Map MOCs/Map.md'}));await waitFor(()=>expect(vaultClient.read).toHaveBeenLastCalledWith('MOCs/Map.md'));expect(document.querySelector('[data-control="vault-moc-open"][aria-current="page"]')).toHaveClass('is-selected');
 fireEvent.click(screen.getByRole('tab',{name:'Search'}));assertTab('Search');expect(document.querySelector('.vault-query')).not.toHaveClass('is-hidden');expect(document.querySelector('.vault-folder')).toBeNull();
 fireEvent.click(document.querySelector('[data-control="search-submit"]')!);await screen.findByRole('button',{name:'Guides/Guide.md Guide'});fireEvent.click(screen.getByRole('button',{name:'Guides/Guide.md Guide'}));await waitFor(()=>expect(document.querySelector('[data-control="search-result-open"][aria-current="page"]')).toHaveClass('is-selected'));
 fireEvent.click(screen.getByRole('tab',{name:'Folders'}));assertTab('Folders');expect(document.querySelector('[data-control="vault-note-open"][aria-current="page"]')).toHaveClass('is-selected');
 fireEvent.click(document.querySelector('[data-control="vault-list-back"]')!);expect(document.querySelector('.vault-search')).not.toHaveClass('is-reading');expect(document.querySelector('[data-control="vault-note-open"][aria-current="page"]')).toHaveClass('is-selected');
});
