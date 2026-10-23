import { expect, it } from 'vitest';
import { safeDiagramSource, sanitizeDiagram } from './diagramSecurity';
it('rejects note-controlled configuration, remote resources and oversized diagrams',()=>{
  expect(safeDiagramSource('flowchart LR\n A-->B')).toBe(true);
  for(const source of ['%%{init: {securityLevel: "loose"}}%%\nflowchart LR\nA-->B','---\nconfig: {}\n---\nflowchart LR\nA-->B','flowchart LR\nA["url(https://example.com)"]','x'.repeat(50001)])expect(safeDiagramSource(source)).toBe(false);
});
it('retains SVG geometry but strips active elements, handlers and linked resources',()=>{
  const safe=sanitizeDiagram('<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><foreignObject><div>HTML</div></foreignObject><image href="https://evil.test/a.png"/><a href="javascript:alert(1)"><text>click</text></a><rect x="1" width="10" onload="alert(1)"/><animate attributeName="href"/></svg>');
  expect(safe).toContain('<rect');
  expect(safe).not.toMatch(/script|foreignObject|image|javascript:|onload|animate|https:\/\/evil/);
});
