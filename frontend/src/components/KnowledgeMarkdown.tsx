import { memo, type ComponentProps } from "react";
import Markdown, { defaultUrlTransform, type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFrontmatter from "remark-frontmatter";

import { VaultImage } from "../features/search/VaultMarkdown";
import { useVaultText } from "../features/search/vaultText";

function KnowledgeImage({ src, alt }: ComponentProps<"img">) {
  const text = useVaultText();
  return src?.startsWith("llm-wiki-image:")
    ? <VaultImage key={src} path="" source={src} alt={alt || text.imageOpen} text={text} />
    : <span role="status">{text.imageError}: {alt}</span>;
}

const components: Components = { img: KnowledgeImage };
const plugins = [remarkGfm, remarkFrontmatter];
const transformUrl = (url: string, key: string) => key === "src" && /^llm-wiki-image:[a-f0-9]{64}$/.test(url) ? url : defaultUrlTransform(url);

/** Metadata stays in the source document, outside the reading surface. */
export const KnowledgeMarkdown = memo(function KnowledgeMarkdown({ children }: { children: string }) {
  return <div className="knowledge-markdown" data-user-content>
    <Markdown remarkPlugins={plugins} skipHtml urlTransform={transformUrl} components={components}>{children}</Markdown>
  </div>;
});
