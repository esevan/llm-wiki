import { createContext, memo, useContext, useEffect, useId, useMemo, useRef, useState, type ComponentProps } from 'react';
import Markdown, { defaultUrlTransform, type Components } from 'react-markdown';
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
  const markup = useMemo(() => ({ __html: svg }), [svg]);
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
      : svg ? <div className="vault-diagram-scroll" tabIndex={0} role="img" aria-label={text.diagram + ': ' + source.slice(0, 120)} title={text.diagramScroll} dangerouslySetInnerHTML={markup} />
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
type VaultRenderContext = {
  path: string;
  notes: VaultNote[];
  onNavigate: (path: string, heading?: string) => void;
  text: VaultText;
};
const RenderContext = createContext<VaultRenderContext | null>(null);
function useRenderContext() {
  const value = useContext(RenderContext);
  if (!value) throw new Error("Vault Markdown renderer requires its context");
  return value;
}
function VaultPre({ children }: ComponentProps<'pre'>) { return <>{children}</>; }
function VaultCode({ className, children }: ComponentProps<'code'>) {
  const { text } = useRenderContext();
  const source = String(children).trim();
  return className === 'language-mermaid' ? <VaultDiagram key={source} source={source} text={text} />
    : className ? <pre><code className={className}>{children}</code></pre> : <code>{children}</code>;
}
function VaultImageRenderer({ src, alt }: ComponentProps<'img'>) {
  const { path, text } = useRenderContext();
  return <VaultImage key={JSON.stringify([path, src])} path={path} source={String(src || '')} alt={alt || ''} text={text} />;
}
function VaultLink({ href, children }: ComponentProps<'a'>) {
  const { path, notes, onNavigate, text } = useRenderContext();
  const target = href ? resolveNoteLink(path, href, notes) : null;
  return target ? <a data-control="vault-note-link" href={'#' + encodeURIComponent(target.heading)} onClick={event => { event.preventDefault(); onNavigate(target.path, target.heading); }}>{children}</a>
    : <span className="vault-unresolved" title={/^[a-z]+:/i.test(href || '') ? text.external : text.unresolved}>{children} ↗</span>;
}
const components: Components = { pre: VaultPre, code: VaultCode, img: VaultImageRenderer, a: VaultLink };
const plugins = [remarkGfm, remarkFrontmatter, remarkVaultLinks];
const mocPlugins = [...plugins, remarkMocRows];
const transformUrl = (url: string, key: string) => key === 'src' && /^llm-wiki-image:[a-f0-9]{64}$/.test(url) ? url : defaultUrlTransform(url);
const VaultMarkdownBody = memo(function VaultMarkdownBody({ markdown, isMoc }: { markdown: string; isMoc: boolean }) {
  return <Markdown remarkPlugins={isMoc ? mocPlugins : plugins} skipHtml urlTransform={transformUrl} components={components}>{markdown}</Markdown>;
});
export function VaultMarkdown({ markdown, path, notes, onNavigate, text, isMoc=false }: { isMoc?: boolean; markdown: string; path: string; notes: VaultNote[]; onNavigate: (path: string, heading?: string) => void; text: VaultText }) {
  return <RenderContext.Provider value={{ path, notes, onNavigate, text }}>
    <div className="vault-markdown" data-user-content><VaultMarkdownBody markdown={markdown} isMoc={isMoc} /></div>
  </RenderContext.Provider>;
}
