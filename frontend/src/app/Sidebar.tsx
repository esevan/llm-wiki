import { useSyncExternalStore } from 'react';
import english from '../../public/i18n/en.json';
import korean from '../../public/i18n/ko.json';
import type { ViewId } from './App';

interface SidebarProps {
  activeView: ViewId;
  onSelectView: (view: ViewId) => void;
}

const navigation = [
  { view: 'workbench', icon: '◫', label: 'nav.workbench' },
  { view: 'compass', icon: '⌁', label: 'nav.compass' },
  { view: 'search', icon: '▤', label: 'nav.search_vault' },
  { view: 'ai-setup', icon: '⚙', label: 'nav.ai_setup' },
] as const;

function subscribeLocale(changed: () => void) {
  const observer = new MutationObserver(changed);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
  return () => observer.disconnect();
}

export function Sidebar({ activeView, onSelectView }: SidebarProps) {
  const locale = useSyncExternalStore(subscribeLocale,
    () => document.documentElement.lang.startsWith('ko') ? 'ko' : 'en', () => 'en');
  const text = locale === 'ko' ? korean : english;
  return (
    <aside className="side">
      <div className="brand">llm wiki<i /></div>
      <nav className="nav" aria-label={text['nav.label']}>
        {navigation.map(({ view, icon, label }) => (
          <button
            className={activeView === view ? 'active' : undefined}
            aria-current={activeView === view ? 'page' : undefined}
            data-view={view}
            data-control={`sidebar-view-${view}`}
            key={view}
            onClick={() => onSelectView(view)}
            type="button"
          >
            <span className="nav-icon" aria-hidden="true">{icon}</span>
            <span className="nav-label">{text[label]}</span>
          </button>
        ))}
      </nav>
      <label className="locale-switch">
        <span>{text['language.label']}</span>
        <select id="locale-select" data-control="locale-select" aria-label={text['language.label']} defaultValue={locale}>
          <option value="en">English</option>
          <option value="ko">한국어</option>
        </select>
      </label>
      <div className="tip"><b>{text['app.control_title']}</b>{text['app.control_body']}</div>
    </aside>
  );
}
