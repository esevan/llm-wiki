import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { KnowledgeMarkdown } from './KnowledgeMarkdown';
import { vaultClient } from '../features/search/vaultClient';
vi.mock('mermaid',()=>({default:{initialize:vi.fn()}}));
vi.mock('../features/search/vaultClient',async original=>({...await original<typeof import('../features/search/vaultClient')>(),vaultClient:{image:vi.fn().mockResolvedValue({dataUrl:'data:image/png;base64,cG5n'})}}));
beforeEach(()=>{document.documentElement.lang='en';vi.mocked(vaultClient.image).mockClear();});
it('renders original images beside draft prose without exposing remote URLs',async()=>{
 const ref='llm-wiki-image:'+'b'.repeat(64);
 render(<KnowledgeMarkdown>{`The diagram explains the decision.\n\n![Decision map](${ref})\n\n![External](https://outside.test/track.png)`}</KnowledgeMarkdown>);
 await screen.findByRole('img',{name:'Decision map'});
 expect(vaultClient.image).toHaveBeenCalledWith('',ref);
 expect(screen.getByText('The diagram explains the decision.')).toBeVisible();
 expect(document.querySelector('img[src^="https:"]')).toBeNull();
});

it('keeps the image and full-size modal across status renders and locale changes',async()=>{
 const ref='llm-wiki-image:'+'a'.repeat(64),body=`![Stable](${ref})`;
 const view=(tick:number)=><div><span>Status {tick}</span><KnowledgeMarkdown>{body}</KnowledgeMarkdown></div>;
 const rendered=render(view(0));const original=await screen.findByRole('img',{name:'Stable'});
 const opener=screen.getByRole('button',{name:'Open full-size image: Stable'});opener.focus();fireEvent.click(opener);
 const dialog=screen.getByRole('dialog');
 for(let tick=1;tick<=3;tick++)rendered.rerender(view(tick));
 expect(document.querySelector('.knowledge-markdown img')).toBe(original);
 expect(screen.getByRole('dialog')).toBe(dialog);expect(vaultClient.image).toHaveBeenCalledTimes(1);
 await act(async()=>{document.documentElement.lang='ko';});
 await screen.findByRole('button',{name:'이미지 닫기 ✕'});
 expect(document.querySelector('.knowledge-markdown img')).toBe(original);
 expect(screen.getByRole('dialog')).toBe(dialog);expect(vaultClient.image).toHaveBeenCalledTimes(1);
 fireEvent.keyDown(document,{key:'Escape'});
 await waitFor(()=>expect(screen.queryByRole('dialog')).toBeNull());expect(opener).toHaveFocus();
});

it('replaces a changed image and ignores a late result from the previous source',async()=>{
 const first='llm-wiki-image:'+'1'.repeat(64),second='llm-wiki-image:'+'2'.repeat(64);
 let finishFirst!:(value:{dataUrl:string})=>void,finishSecond!:(value:{dataUrl:string})=>void;
 vi.mocked(vaultClient.image).mockImplementationOnce(()=>new Promise(resolve=>{finishFirst=resolve;})).mockImplementationOnce(()=>new Promise(resolve=>{finishSecond=resolve;}));
 const rendered=render(<KnowledgeMarkdown>{`![First](${first})`}</KnowledgeMarkdown>);
 rendered.rerender(<KnowledgeMarkdown>{`![Second](${second})`}</KnowledgeMarkdown>);
 await act(async()=>finishSecond({dataUrl:'data:image/png;base64,c2Vjb25k'}));
 const current=screen.getByRole('img',{name:'Second'});expect(current).toHaveAttribute('src','data:image/png;base64,c2Vjb25k');
 await act(async()=>finishFirst({dataUrl:'data:image/png;base64,Zmlyc3Q='}));
 expect(screen.getByRole('img',{name:'Second'})).toBe(current);expect(screen.queryByRole('img',{name:'First'})).toBeNull();
 expect(vaultClient.image).toHaveBeenCalledTimes(2);
});
