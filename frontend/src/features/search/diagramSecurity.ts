import DOMPurify from 'dompurify';
export function safeDiagramSource(source: string) {
  return source.length <= 50000 && !/%%\s*\{|^\s*---|(?:https?:|javascript:|data:|url\s*\(|@import)/im.test(source);
}
export function sanitizeDiagram(svg: string) {
  return DOMPurify.sanitize(svg, { USE_PROFILES: { svg: true, svgFilters: true },
    FORBID_TAGS: ['a', 'image', 'foreignObject', 'script', 'animate', 'animateMotion', 'animateTransform', 'set'],
    FORBID_ATTR: ['href', 'xlink:href', 'src', 'onload', 'onclick'],
  });
}
