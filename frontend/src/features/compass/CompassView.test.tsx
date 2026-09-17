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
