import { useVaultText } from '../features/search/vaultText';
import type { ViewId } from './App';

interface SidebarProps {
  activeView: ViewId;
  onSelectView: (view: ViewId) => void;
}

const navigation: ReadonlyArray<[ViewId, string]> = [
  ['workbench', '✦ Workbench'],
  ['search', '⌕ Search vault'],
  ['compass', '◒ Compass'],
  ['ai-setup', '⚙ AI setup'],
];

export function Sidebar({ activeView, onSelectView }: SidebarProps) {
  const text = useVaultText();
  const items = activeView === 'search' ? [navigation[0], navigation[2], navigation[1], navigation[3]] : navigation;
  const vaultLabels: Partial<Record<ViewId, string>> = { workbench: '◫ ' + text.shellWorkbench, compass: '⌁ ' + text.shellCompass, search: '▤ ' + text.title, 'ai-setup': '⚙ ' + text.shellSettings };
  return (
    <aside className="side">
      <div className="brand">llm wiki<i /></div>
      <nav className="nav">
        {items.map(([view, label]) => (
          <button
            className={activeView === view ? 'active' : undefined}
            aria-current={activeView === view ? 'page' : undefined}
            data-view={view}
            data-control={`sidebar-view-${view}`}
            key={view}
            onClick={() => onSelectView(view)}
            type="button"
          >
            {activeView === 'search' ? vaultLabels[view] : label}
          </button>
        ))}
      </nav>
      <label className="locale-switch">
        <span>Language</span>
        <select id="locale-select" data-control="locale-select" aria-label="Language" defaultValue="en">
          <option value="en">English</option>
          <option value="ko">한국어</option>
        </select>
      </label>
      <div className="tip"><b>You’re in control.</b>Nothing is approved or moved forward until you choose it.</div>
    </aside>
  );
}
