import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useState } from 'react';

import { Sidebar } from './Sidebar';
import type { ViewId } from './App';
import english from '../../public/i18n/en.json';
import korean from '../../public/i18n/ko.json';
import foundation from '../../public/runtime/foundation.js?raw';

function SidebarHarness() {
  const [view, setView] = useState<ViewId>('workbench');
  return <div className="app"><Sidebar activeView={view} onSelectView={setView}/><main><output>{view}</output></main></div>;
}

const originalClient = window.llmWikiApplication;

afterEach(() => {
  document.documentElement.lang = 'en';
  window.llmWikiApplication = originalClient;
});

const routes = ['workbench', 'compass', 'search', 'ai-setup'] as const;
const labels = {
  en: ['Workbench', 'Compass', 'Search vault', 'AI setup'],
  ko: ['워크벤치', '나침반', 'Vault 검색', 'AI 설정'],
};

describe('Shared Sidebar navigation', () => {
  it.each(['en', 'ko'] as const)('retains the same menu, icons, shell position and focused controls on every route in %s', async locale => {
    document.documentElement.lang = locale;
    render(<SidebarHarness />);
    const rail = document.querySelector('.side')!;
    const nav = screen.getByRole('navigation');
    const controls = within(nav).getAllByRole('button');
    const icons = controls.map(button => button.querySelector('[aria-hidden="true"]')?.textContent);

    for (const [index, route] of routes.entries()) {
      controls[index].focus();
      fireEvent.click(controls[index]);
      expect(screen.getByRole('status')).toHaveTextContent(route);
      expect(document.querySelector('.app')?.firstElementChild).toBe(rail);
      expect(rail.nextElementSibling?.tagName).toBe('MAIN');
      expect(screen.getByRole('navigation')).toBe(nav);
      expect(within(nav).getAllByRole('button')).toEqual(controls);
      expect(controls.map(button => button.dataset.view)).toEqual(routes);
      expect(controls.map(button => button.querySelector('.nav-label')?.textContent)).toEqual(labels[locale]);
      expect(controls.map(button => button.querySelector('[aria-hidden="true"]')?.textContent)).toEqual(icons);
      expect(nav.querySelectorAll('[aria-current="page"]')).toHaveLength(1);
      expect(controls[index]).toHaveAttribute('aria-current', 'page');
      expect(controls[index]).toHaveFocus();
      expect(controls.every(button => button.getAttribute('type') === 'button' && button.tabIndex === 0)).toBe(true);
    }

    const focused = controls[2];
    focused.focus();
    const next = locale === 'en' ? 'ko' : 'en';
    await act(async () => { document.documentElement.lang = next; });
    await waitFor(() => expect(within(nav).getByRole('button', { name: labels[next][2] })).toBe(focused));
    expect(focused).toHaveFocus();
    expect(screen.getByRole('status')).toHaveTextContent('ai-setup');
    expect(controls[3]).toHaveAttribute('aria-current', 'page');
  });

  it('uses the real locale selector/runtime persistence path without replacing the rail or losing selection', async () => {
    render(<SidebarHarness />);
    const request = vi.fn().mockImplementation(async ({ path, body }: { path: string; body?: string }) => ({
      ok: true, status: 200,
      json: async () => path === '/i18n/en' ? english : path === '/i18n/ko' ? korean : { locale: body ? JSON.parse(body).locale : 'en' },
    }));
    window.llmWikiApplication = { request };
    // Use the production locale bootstrap and change handler, before unrelated
    // refinement setup begins. Board/dashboard refreshes are isolated fixtures.
    const source = foundation.slice(0, foundation.indexOf('const refinementPreviewFailureMessage')) + foundation.slice(foundation.indexOf('async function api('));
    const runtime = new Function('loadBoard', 'loadDashboard', 'chatTarget', 'showNotice', `${source}; return { initializeLocale, stop: () => localeObserver.disconnect() };`)(vi.fn(), vi.fn(), null, vi.fn());
    try {
      await act(async () => { await runtime.initializeLocale(); });
      const rail = document.querySelector('.side');
      const controls = within(screen.getByRole('navigation')).getAllByRole('button');
      for (const [index, route] of routes.entries()) {
        fireEvent.click(controls[index]);
        for (const locale of ['ko', 'en'] as const) {
          fireEvent.change(screen.getByRole('combobox'), { target: { value: locale } });
          await waitFor(() => expect(document.documentElement.lang).toBe(locale));
          await waitFor(() => expect(controls.map(button => button.querySelector('.nav-label')?.textContent)).toEqual(labels[locale]));
          expect(screen.getByRole('combobox')).toHaveValue(locale);
          expect(screen.getByRole('status')).toHaveTextContent(route);
          expect(document.querySelector('.side')).toBe(rail);
          expect(controls[index]).toHaveAttribute('aria-current', 'page');
        }
      }
      expect(request.mock.calls.filter(([arg]) => arg.method === 'PUT').map(([arg]) => JSON.parse(arg.body).locale)).toEqual(['ko', 'en', 'ko', 'en', 'ko', 'en', 'ko', 'en']);
    } finally {
      runtime.stop();
    }
  });
});
