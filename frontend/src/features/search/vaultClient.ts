export interface VaultNote { path: string; title: string; isMoc: boolean; modifiedAt: number | null; managed: boolean; readable: boolean }
export interface VaultListing { notes: VaultNote[]; folders: string[] }
export interface VaultDocument { path: string; markdown: string; canonical_locale: string; served_locale: string; translated: boolean }
export interface VaultResult { path: string; title: string; snippet?: string }
async function request<T>(path: string): Promise<T> {
  const response = await window.llmWikiApplication.request({ path });
  if (!response.ok) throw new Error('Vault request failed');
  return response.json<T>();
}
export const vaultClient = {
  browse: () => request<VaultListing>('/vault/browse'),
  // Always read canonical/original bytes. This reader never implicitly translates or writes.
  read: (path: string) => request<VaultDocument>('/knowledge?path=' + encodeURIComponent(path) + '&locale=en&translate=false'),
  image: async (path: string, source: string) => request<{ dataUrl: string }>('/vault/image?path=' + encodeURIComponent(path) + '&source=' + encodeURIComponent(decodeURIComponent(source))),
  search: (query: string, semantic: boolean, offset = 0) => request<{ results: VaultResult[]; has_more?: boolean }>('/search?q=' + encodeURIComponent(query) + '&limit=20&offset=' + offset + (semantic ? '&semantic=true' : '')),
};

export function resolveNoteLink(source: string, target: string, notes: VaultNote[]): { path: string; heading: string } | null {
  let decoded: string;
  try { decoded = decodeURIComponent(target); } catch { return null; }
  if (/^[a-z][a-z\d+.-]*:|^[\\/]|[\\\0?]/i.test(decoded)) return null;
  const [name, heading = ''] = decoded.split('#', 2);
  if (!name) return { path: source, heading };
  const normalize = (value: string) => {
    const segments: string[] = [];
    for (const part of value.split('/')) {
      if (part === '..') { if (!segments.length) return null; segments.pop(); }
      else if (part && part !== '.') segments.push(part);
    }
    return segments.join('/');
  };
  const withExtension = /\.md$/i.test(name) ? name : name + '.md';
  const parent = source.split('/').slice(0, -1).join('/');
  const relative = normalize((parent ? parent + '/' : '') + withExtension);
  const root = normalize(withExtension);
  const exact = notes.find(note => note.path === relative) ?? notes.find(note => note.path === root);
  if (exact) return { path: exact.path, heading };
  // Obsidian short links are accepted only when unambiguous across the Vault.
  if (!name.includes('/')) {
    const matches = notes.filter(note => note.path.split('/').at(-1) === withExtension);
    if (matches.length === 1) return { path: matches[0].path, heading };
  }
  return null;
}

interface MarkdownNode { type: string; value?: string; url?: string; children?: MarkdownNode[] }
/** Transform text nodes only: code fences and inline code stay literal. */
export function remarkVaultLinks() {
  return (tree: MarkdownNode) => {
    const visit = (node: MarkdownNode) => {
      if (!node.children || ['link', 'code', 'inlineCode', 'html'].includes(node.type)) return;
      node.children = node.children.flatMap(child => {
        if (child.type !== 'text' || !child.value) { visit(child); return [child]; }
        const result: MarkdownNode[] = []; let end = 0;
        for (const match of child.value.matchAll(/\[\[([^\]\n]+)\]\]/g)) {
          if (match.index! > end) result.push({ type: 'text', value: child.value.slice(end, match.index) });
          const [target, alias] = match[1].split('|', 2);
          result.push({ type: 'link', url: target, children: [{ type: 'text', value: alias || target.split('#')[0] || target }] });
          end = match.index! + match[0].length;
        }
        if (end < child.value.length) result.push({ type: 'text', value: child.value.slice(end) });
        return result;
      });
    };
    visit(tree);
  };
}

/** MOC rows keep authored link labels and optional following descriptions together. */
export function remarkMocRows() {
  return (tree: MarkdownNode) => {
    const children=tree.children??[];const result:MarkdownNode[]=[];
    const isLink=(node:MarkdownNode)=>node.type==='paragraph'&&node.children?.length===1&&node.children[0].type==='link';
    for(let index=0;index<children.length;index++){
      const node=children[index];
      if(!isLink(node)){result.push(node);continue;}
      const row:MarkdownNode & {data:unknown}={type:'vaultMocRow',children:[node],data:{hName:'div',hProperties:{className:['vault-moc-link']}}};
      const next=children[index+1];
      if(next?.type==='paragraph'&&!isLink(next)){row.children!.push(next);index++;}
      result.push(row);
    }
    tree.children=result;
  };
}
export function noteLinkTargets(markdown:string):string[]{
  const prose=markdown.replace(/^\s*(`{3,}|~{3,})[^\n]*\n[\s\S]*?^\s*\1\s*$/gm,'').replace(/`[^`]*`/g,'');
  return Array.from(prose.matchAll(/\[\[([^\]|\n]+)(?:\|[^\]\n]+)?\]\]|\[[^\]]+\]\(([^)]+)\)/g)).map(match=>match[1]||match[2]);
}
