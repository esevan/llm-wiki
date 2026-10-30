import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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
beforeEach(()=>{document.documentElement.lang='en';vi.mocked(vaultClient.image).mockReset().mockResolvedValue({dataUrl:image});});
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
