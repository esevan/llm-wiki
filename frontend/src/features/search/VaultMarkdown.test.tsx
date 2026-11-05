import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import mermaid from 'mermaid';
import { VaultMarkdown } from './VaultMarkdown';
import { vaultClient, type VaultNote } from './vaultClient';
import { useVaultText } from './vaultText';
vi.mock('mermaid',()=>({default:{initialize:vi.fn(),render:vi.fn().mockResolvedValue({svg:'<svg xmlns="http://www.w3.org/2000/svg"><text>A to B</text><path d="M0 0L1 1"/></svg>'})}}));
vi.mock('./vaultClient',async importOriginal=>({...await importOriginal<typeof import('./vaultClient')>(),vaultClient:{image:vi.fn()}}));
const image='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j8ioAAAAASUVORK5CYII=';
const notes:VaultNote[]=[{path:'Guide.md',title:'Guide',isMoc:false,modifiedAt:1,managed:true,readable:true},{path:'Map.md',title:'Map',isMoc:true,modifiedAt:1,managed:true,readable:true}];
function Fixture({body,navigate=()=>{}}:{body:string;navigate?:(path:string,heading?:string)=>void}){return <VaultMarkdown markdown={body} path="Guide.md" notes={notes} onNavigate={navigate} text={useVaultText()}/>;}
beforeEach(()=>{document.documentElement.lang='en';vi.mocked(mermaid.render).mockClear();vi.mocked(vaultClient.image).mockReset().mockResolvedValue({dataUrl:image});});
it('renders Markdown and asks the actual renderer API for Mermaid, while skipping embedded HTML',async()=>{
 render(<Fixture body={'# Heading\n\n**Bold** and `inline`.\n\n```mermaid\nflowchart LR\nA-->B\n```\n\n<script>alert(1)</script>\n\n<iframe src="https://evil.test"/>'}/>);
 expect(screen.getByRole('heading',{name:'Heading'})).toBeVisible();expect(screen.getByText('Bold').tagName).toBe('STRONG');await screen.findByText('A to B');
 expect(mermaid.render).toHaveBeenCalledWith(expect.stringMatching(/^vault-diagram-/),'flowchart LR\nA-->B');expect(document.querySelector('script,iframe')).toBeNull();
 fireEvent.click(screen.getByText('View source'));expect(document.querySelector('details')).toHaveAttribute('open');
});
it('shows escaped source when Mermaid rejects invalid syntax',async()=>{
 vi.mocked(mermaid.render).mockRejectedValueOnce(new Error('parse failed'));
 render(<Fixture body={'```mermaid\ninvalid <script>\n```'}/>);await screen.findByText('Could not render diagram');expect(document.querySelector('script')).toBeNull();expect(document.querySelector('code')).toHaveTextContent('invalid <script>');
});
it('loads note-relative images, opens a focus-trapped full-size view, and retries inline failures',async()=>{
 vi.mocked(vaultClient.image).mockRejectedValueOnce(new Error('missing'));
 render(<Fixture body={'![An example](assets/example.png)'}/>);await screen.findByText('Could not load image');fireEvent.click(screen.getByText('Try again'));await screen.findByRole('img',{name:'An example'});
 expect(vaultClient.image).toHaveBeenLastCalledWith('Guide.md','assets/example.png');fireEvent.click(screen.getByRole('button',{name:'Open full-size image: An example'}));expect(screen.getByRole('dialog')).toBeVisible();fireEvent.click(screen.getByRole('button',{name:'Close image ✕'}));expect(screen.queryByRole('dialog')).toBeNull();
});
it('follows aliased wikilinks and Markdown headings but leaves unsafe or ambiguous links inert',async()=>{
 const navigate=vi.fn();render(<Fixture body={'[[Map#Start|Read the map]] and [heading](#Heading) and [external](https://evil.test).\n\n[unsafe](javascript:alert(1))'} navigate={navigate}/>);
 fireEvent.click(screen.getByRole('link',{name:'Read the map'}));expect(navigate).toHaveBeenCalledWith('Map.md','Start');fireEvent.click(screen.getByRole('link',{name:'heading'}));expect(navigate).toHaveBeenCalledWith('Guide.md','Heading');expect(screen.queryByRole('link',{name:'external'})).toBeNull();expect(document.querySelector('[href^="javascript:"]')).toBeNull();
 await waitFor(()=>expect(screen.getByText(/external/)).toHaveAttribute('title'));
});
it('resolves frozen Knowledge image refs locally after publication and leaves invalid schemes inert',async()=>{
 const ref='llm-wiki-image:'+ 'a'.repeat(64);
 render(<Fixture body={`Diagram context.\n\n![Architecture](${ref})\n\n![Unsafe](javascript:alert)`}/>);
 await screen.findByRole('img',{name:'Architecture'});
 expect(vaultClient.image).toHaveBeenCalledWith('Guide.md',ref);
 expect(document.querySelector('img[src^="llm-wiki-image:"]')).toBeNull();
 expect(document.querySelector('[src^="javascript:"]')).toBeNull();
});

it('retains image, diagram, disclosure and open modal through irrelevant updates',async()=>{
 const body='![Stable](assets/stable.png)\n\n```mermaid\nflowchart LR\nA-->B\n```';
 const rendered=render(<Fixture body={body}/>);const original=await screen.findByRole('img',{name:'Stable'});await screen.findByText('A to B');
 const diagram=document.querySelector('.vault-diagram-scroll');const svg=diagram?.querySelector('svg');const disclosure=document.querySelector('details')!;fireEvent.click(screen.getByText('View source'));
 const opener=screen.getByRole('button',{name:'Open full-size image: Stable'});opener.focus();fireEvent.click(opener);const dialog=screen.getByRole('dialog');
 for(let i=0;i<3;i++)rendered.rerender(<Fixture body={body} navigate={()=>{}}/>);
 expect(document.querySelector('.vault-markdown img')).toBe(original);expect(document.querySelector('.vault-diagram-scroll')).toBe(diagram);expect(diagram?.querySelector('svg')).toBe(svg);expect(disclosure).toHaveAttribute('open');expect(screen.getByRole('dialog')).toBe(dialog);
 expect(vaultClient.image).toHaveBeenCalledTimes(1);expect(mermaid.render).toHaveBeenCalledTimes(1);
 await act(async()=>{document.documentElement.lang='ko';});
 const close=await screen.findByRole('button',{name:'이미지 닫기 ✕'});expect(screen.getByText('원본 보기')).toBeVisible();
 expect(document.querySelector('.vault-diagram-scroll')).toBe(diagram);expect(diagram?.querySelector('svg')).toBe(svg);expect(mermaid.render).toHaveBeenCalledTimes(1);expect(vaultClient.image).toHaveBeenCalledTimes(1);
 fireEvent.click(close);await waitFor(()=>expect(screen.queryByRole('dialog')).toBeNull());expect(opener).toHaveFocus();
});

function ContextFixture({path,body,navigate=()=>{},listing=notes}:{path:string;body:string;navigate?:(path:string,heading?:string)=>void;listing?:VaultNote[]}){
 return <VaultMarkdown markdown={body} path={path} notes={listing} onNavigate={navigate} text={useVaultText()}/>;
}
it('updates document image identity and current link context without stale callbacks',async()=>{
 const first=vi.fn(),next=vi.fn(),body='![Local](assets/example.png)\n\n[[Map#Start|Read map]]';
 const rendered=render(<ContextFixture path="Guide.md" body={body} navigate={first}/>);const original=await screen.findByRole('img',{name:'Local'});
 rendered.rerender(<ContextFixture path="Guide.md" body={body} navigate={next} listing={[...notes]}/>);
 fireEvent.click(screen.getByRole('link',{name:'Read map'}));expect(next).toHaveBeenCalledWith('Map.md','Start');expect(first).not.toHaveBeenCalled();expect(vaultClient.image).toHaveBeenCalledTimes(1);
 rendered.rerender(<ContextFixture path="Other.md" body={body} navigate={next}/>);
 const changed=await screen.findByRole('img',{name:'Local'});expect(changed).not.toBe(original);expect(vaultClient.image).toHaveBeenLastCalledWith('Other.md','assets/example.png');expect(vaultClient.image).toHaveBeenCalledTimes(2);
 rendered.rerender(<ContextFixture path="Other.md" body={body} listing={[]} navigate={next}/>);
 expect(screen.queryByRole('link',{name:'Read map'})).toBeNull();expect(vaultClient.image).toHaveBeenCalledTimes(2);
});

it('keeps an image error stable and retries without treating a status rerender as retry',async()=>{
 vi.mocked(vaultClient.image).mockRejectedValueOnce(new Error('missing'));
 const body='![Unavailable](missing.png)',rendered=render(<Fixture body={body}/>);const error=await screen.findByText('Could not load image');
 rendered.rerender(<Fixture body={body}/>);expect(screen.getByText('Could not load image')).toBe(error);expect(vaultClient.image).toHaveBeenCalledTimes(1);
 fireEvent.click(screen.getByText('Try again'));await screen.findByRole('img',{name:'Unavailable'});expect(vaultClient.image).toHaveBeenCalledTimes(2);
});

it('ignores stale image errors after document navigation and closes the old overlay',async()=>{
 let rejectOld!:(error:Error)=>void;
 vi.mocked(vaultClient.image).mockImplementationOnce(()=>new Promise((_resolve,reject)=>{rejectOld=reject;}));
 const body='![Local](assets/image.png)',rendered=render(<ContextFixture path="Old.md" body={body}/>);
 rendered.rerender(<ContextFixture path="New.md" body={body}/>);await screen.findByRole('img',{name:'Local'});
 await act(async()=>rejectOld(new Error('late old failure')));expect(screen.queryByText('Could not load image')).toBeNull();
 fireEvent.click(screen.getByRole('button',{name:'Open full-size image: Local'}));expect(screen.getByRole('dialog')).toBeVisible();
 rendered.rerender(<ContextFixture path="Newest.md" body={body}/>);await waitFor(()=>expect(screen.queryByRole('dialog')).toBeNull());await screen.findByRole('img',{name:'Local'});
});

it('renders a new diagram after failure and ignores a late old diagram result',async()=>{
 let finishOld!:(value:Awaited<ReturnType<typeof mermaid.render>>)=>void;
 vi.mocked(mermaid.render).mockImplementationOnce(()=>new Promise(resolve=>{finishOld=resolve;}));
 const rendered=render(<Fixture body={'```mermaid\nflowchart LR\nOld-->Pending\n```'}/>);
 rendered.rerender(<Fixture body={'```mermaid\nflowchart LR\nNew-->Current\n```'}/>);await screen.findByText('A to B');const current=document.querySelector('.vault-diagram-scroll');
 await act(async()=>finishOld({svg:'<svg xmlns="http://www.w3.org/2000/svg"><text>Old diagram</text></svg>',diagramType:"flowchart-v2"}));expect(document.querySelector('.vault-diagram-scroll')).toBe(current);expect(screen.queryByText('Old diagram')).toBeNull();
 vi.mocked(mermaid.render).mockRejectedValueOnce(new Error('invalid'));
 rendered.rerender(<Fixture body={'```mermaid\ninvalid diagram\n```'}/>);await screen.findByText('Could not render diagram');
 rendered.rerender(<Fixture body={'```mermaid\nflowchart LR\nRecovered-->Success\n```'}/>);await screen.findByText('A to B');expect(screen.queryByText('Could not render diagram')).toBeNull();expect(mermaid.render).toHaveBeenCalledTimes(4);
});
