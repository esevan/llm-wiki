import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { CompassView } from './CompassView';

const reply = (body: unknown, ok = true) => ({ ok, status: ok ? 200 : 500, json: async () => body });
describe('Compass completed tasks', () => {
  it('lists every completion including children in completion order and reloads on return', async () => {
    const items = Array.from({ length: 7 }, (_, i) => ({ kind: 'task', id: `t${i}`, title: `Completed ${i}`, state: 'completed', taskRevision: 1, completedAt: `2026-09-${10 + i}T00:00:00Z`, parentTaskId: i === 6 ? 't0' : undefined }));
    const request = vi.fn().mockResolvedValue(reply({ categories: [{ items }] }));
    window.llmWikiApplication = { request };
    const view = render(<CompassView active />);
    fireEvent.click(screen.getByRole('button', { name: 'Completed tasks' }));
    await screen.findByText('Completed 6');
    expect([...document.querySelectorAll('.completed-task-list li')].map(row => row.textContent)).toHaveLength(7);
    expect(document.querySelector('.completed-task-list li')).toHaveTextContent('Completed 6');
    view.rerender(<CompassView active={false} />);
    items.pop();
    view.rerender(<CompassView active />);
    await waitFor(() => expect(screen.queryByText('Completed 6')).toBeNull());
    fireEvent.click(screen.getByRole('button', { name: 'Direction' }));
    expect(document.querySelector('#goal-form')).toBeVisible();
  });
  it('recovers a failed completed list and shows its empty state', async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValueOnce(reply({ error: 'Offline' }, false)).mockResolvedValue(reply({ categories: [] })) };
    render(<CompassView active />);
    fireEvent.click(screen.getByRole('button', { name: 'Completed tasks' }));
    await screen.findByText('Offline');
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    await screen.findByText('No completed tasks yet.');
  });
});

describe('Compass shared Task modal navigation', () => {
  const items = ['first', 'second'].map(id => ({ kind: 'task', id, title: `Completed ${id}`, state: 'completed', taskRevision: 2, completedAt: '2026-10-01T00:00:00Z', workLog: [{ id: `log-${id}`, body: `Evidence ${id}`, comments: [] }], checklist: [], decisions: [], readinessEntries: [] }));
  const setup = (missing = false) => {
    const request = vi.fn(async ({ path }: { path: string }) => {
      if (path === '/workbench') return reply({ categories: [{ items }] });
      if (path.endsWith('/knowledge')) return reply({ pointers: {}, versions: [], ideas: [] });
      if (path.endsWith('/lineage')) return reply({ nodes: [], edges: [], unresolved: [] });
      const task = items.find(item => path === `/tasks/${item.id}`);
      return task ? reply(missing ? { error: 'Task no longer available' } : task, !missing) : reply({});
    });
    window.llmWikiApplication = { request: request as never };
    render(<CompassView active />);
    fireEvent.click(screen.getByRole('button', { name: 'Completed tasks' }));
    return request;
  };
  it('opens exact completed Tasks on Work and reuses Review and normal modal dismissal', async () => {
    const request = setup();
    const first = await screen.findByRole('button', { name: /Completed first/ });
    first.focus();
    fireEvent.click(first);
    await screen.findByRole('dialog', { name: 'Completed first' });
    expect(screen.getByRole('tab', { name: 'Work', selected: true })).toBeVisible();
    expect(await screen.findByText('Evidence first')).toBeVisible();
    fireEvent.click(first);
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    // The shared Task backdrop intentionally does not discard work.
    fireEvent.click(document.querySelector('.task-detail-modal-backdrop')!);
    expect(screen.getByRole('dialog')).toBeVisible();
    fireEvent.click(screen.getByRole('tab', { name: 'Sessions' }));
    expect(screen.getByRole('tab', { name: 'Sessions', selected: true })).toBeVisible();
    fireEvent.click(screen.getByRole('tab', { name: 'Review' }));
    await waitFor(() => expect(request.mock.calls.some(([input]) => input.path === '/tasks/first/knowledge')).toBe(true));
    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    await waitFor(() => expect(first).toHaveFocus());
    fireEvent.click(screen.getByRole('button', { name: /Completed second/ }));
    await screen.findByRole('dialog', { name: 'Completed second' });
    expect(await screen.findByText('Evidence second')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Close Task detail' }));
    await waitFor(() => expect(screen.getByRole('button', { name: /Completed second/ })).toHaveFocus());
    expect(request.mock.calls.every(([input]) => !(input as { method?: string }).method || (input as { method?: string }).method === 'GET')).toBe(true);
  });
  it('shows the shared missing Task error without inventing another Task or opening Knowledge', async () => {
    const request = setup(true);
    const trigger = await screen.findByRole('button', { name: /Completed first/ });
    fireEvent.click(trigger);
    await screen.findByText('Task no longer available');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeVisible();
    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(request.mock.calls.some(([input]) => input.path.includes('/knowledge'))).toBe(false);
  });
});
