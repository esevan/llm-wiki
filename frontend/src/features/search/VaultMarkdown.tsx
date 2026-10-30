import { useEffect, useId, useRef, useState } from 'react';
import Markdown, { defaultUrlTransform } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkFrontmatter from 'remark-frontmatter';
import { safeDiagramSource, sanitizeDiagram } from './diagramSecurity';
import mermaid from 'mermaid';
import { createPortal } from 'react-dom';
import { useModalInteraction } from '../workbench/useModalInteraction';
import { remarkVaultLinks, remarkMocRows, resolveNoteLink, vaultClient, type VaultNote } from './vaultClient';
import type { VaultText } from './vaultText';

mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', suppressErrorRendering: true, maxTextSize: 50000, maxEdges: 500,
  theme: 'base', htmlLabels: false, flowchart: { htmlLabels: false, useMaxWidth: false, nodeSpacing: 36, rankSpacing: 44 },
  themeVariables: { fontFamily: 'Nunito, Noto Sans KR Variable, sans-serif', fontSize: '15px', primaryColor: '#ffffff', primaryTextColor: '#27242a', primaryBorderColor: '#ded6da', lineColor: '#675e63', secondaryColor: '#fff0f6', tertiaryColor: '#e9f9f1' },
  secure: ['securityLevel', 'startOnLoad', 'maxTextSize', 'maxEdges', 'suppressErrorRendering', 'htmlLabels', 'themeCSS', 'themeVariables'],
});
// Mermaid has global configuration and a shared render queue; never bind generated callbacks.
export function VaultDiagram({ source, text }: { source: string; text: VaultText }) {
  const id = useId().replace(/[^a-z\d]/gi, '');
  const [svg, setSvg] = useState(''); const [error, setError] = useState(false);
  useEffect(() => {
    let cancelled = false;
    if (!safeDiagramSource(source)) return;
    void mermaid.render('vault-diagram-' + id, source).then(result => {
      if (!cancelled) setSvg(sanitizeDiagram(result.svg));
    }).catch(() => { if (!cancelled) setError(true); });
    return () => { cancelled = true; };
  }, [id, source]);
  return <figure className="vault-diagram">
    {error || !safeDiagramSource(source) ? <div className="vault-inline-error" role="status"><strong>{text.diagramError}</strong><p>{text.diagramHint}</p></div>
      : svg ? <div className="vault-diagram-scroll" tabIndex={0} role="img" aria-label={text.diagram + ': ' + source.slice(0, 120)} title={text.diagramScroll} dangerouslySetInnerHTML={{ __html: svg }} />
        : <p role="status">{text.diagramLoading}</p>}
    <figcaption title={text.diagramScroll}>{text.diagram}</figcaption>
    <details data-control="vault-diagram-details"><summary data-control="vault-diagram-source">{text.showSource}</summary><pre><code>{source}</code></pre></details>
  </figure>;
}
function FullImage({ src, alt, close, text }: { src: string; alt: string; close: () => void; text: VaultText }) {
  const panel=useRef<HTMLDivElement>(null); const closeButton=useRef<HTMLButtonElement>(null);
  useModalInteraction(panel,40,close);
  useEffect(() => { const previous=document.activeElement as HTMLElement; closeButton.current?.focus(); return () => previous?.focus(); },[]);
  return createPortal(<div className="vault-image-overlay" role="presentation" onClick={close}><div ref={panel} role="dialog" aria-modal="true" aria-label={alt || text.imageOpen} onClick={event=>event.stopPropagation()}><button data-control="vault-image-close" ref={closeButton} onClick={close}>{text.close} ✕</button><img src={src} alt={alt}/></div></div>,document.body);
}
export function VaultImage({ path, source, alt, text }: { path: string; source: string; alt: string; text: VaultText }) {
  const [data, setData]=useState(''); const [error,setError]=useState(false); const [retry,setRetry]=useState(0); const [open,setOpen]=useState(false);
  useEffect(()=>{let cancelled=false; void vaultClient.image(path,source).then(result=>{if(!cancelled)setData(result.dataUrl);}).catch(()=>{if(!cancelled)setError(true);});return()=>{cancelled=true;};},[path,source,retry]);
  if (error) return <span className="vault-inline-error" role="status"><strong>{text.imageError}</strong><span>{alt || source}</span><span>{text.imageHint}</span><button data-control="vault-image-retry" onClick={()=>{setError(false);setRetry(value=>value+1);}}>{text.retry}</button></span>;
  if (!data) return <span role="status">{text.imageLoading}</span>;
  return <><button data-control="vault-image-open" className="vault-image-button" aria-label={text.imageOpen + ': ' + alt} onClick={()=>setOpen(true)}><img src={data} alt={alt} onError={()=>setError(true)}/></button>{open&&<FullImage src={data} alt={alt} text={text} close={()=>setOpen(false)}/>}</>;
}
export function VaultMarkdown({ markdown, path, notes, onNavigate, text, isMoc=false }: { isMoc?: boolean; markdown: string; path: string; notes: VaultNote[]; onNavigate: (path: string, heading?: string) => void; text: VaultText }) {
  return <div className="vault-markdown" data-user-content><Markdown remarkPlugins={[remarkGfm,remarkFrontmatter,remarkVaultLinks,...(isMoc?[remarkMocRows]:[])]} skipHtml urlTransform={(url,key)=>key==="src" && /^llm-wiki-image:[a-f0-9]{64}$/.test(url)?url:defaultUrlTransform(url)} components={{
    pre: ({ children })=><>{children}</>,
    code: ({ className, children })=>className==='language-mermaid'?<VaultDiagram key={String(children)} source={String(children).trim()} text={text}/>:className?<pre><code className={className}>{children}</code></pre>:<code>{children}</code>,
    img: ({ src, alt })=><VaultImage key={path+String(src)} path={path} source={String(src||'')} alt={alt||''} text={text}/>,
    a: ({ href, children })=>{
      const target=href?resolveNoteLink(path,href,notes):null;
      return target?<a data-control="vault-note-link" href={'#'+encodeURIComponent(target.heading)} onClick={event=>{event.preventDefault();onNavigate(target.path,target.heading);}}>{children}</a>:<span className="vault-unresolved" title={/^[a-z]+:/i.test(href||'')?text.external:text.unresolved}>{children} ↗</span>;
    },
  }}>{markdown}</Markdown></div>;
}
