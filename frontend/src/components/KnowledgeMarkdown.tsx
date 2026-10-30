import Markdown, { defaultUrlTransform } from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFrontmatter from "remark-frontmatter";

import { VaultImage } from "../features/search/VaultMarkdown";
import { useVaultText } from "../features/search/vaultText";

/** Metadata stays in the source document, outside the reading surface. */
export function KnowledgeMarkdown({ children }: { children: string }) {
  const text = useVaultText();
  return <div className="knowledge-markdown" data-user-content>
    <Markdown remarkPlugins={[remarkGfm, remarkFrontmatter]} skipHtml urlTransform={(url,key)=>key==="src" && /^llm-wiki-image:[a-f0-9]{64}$/.test(url)?url:defaultUrlTransform(url)} components={{
      img: ({src,alt}) => src?.startsWith("llm-wiki-image:") ? <VaultImage key={src} path="" source={src} alt={alt || text.imageOpen} text={text}/> : <span role="status">{text.imageError}: {alt}</span>,
    }}>{children}</Markdown>
  </div>;
}
