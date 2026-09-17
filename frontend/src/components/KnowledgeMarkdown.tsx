import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFrontmatter from "remark-frontmatter";

/** Metadata stays in the source document, outside the reading surface. */
export function KnowledgeMarkdown({ children }: { children: string }) {
  return <div className="knowledge-markdown" data-user-content>
    <Markdown remarkPlugins={[remarkGfm, remarkFrontmatter]} skipHtml>{children}</Markdown>
  </div>;
}
