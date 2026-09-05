import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useState } from 'react';

import { Sidebar } from './Sidebar';
import type { ViewId } from './App';

function SidebarHarness() {
  const [view, setView] = useState<ViewId>('workbench');
  return <><Sidebar activeView={view} onSelectView={setView}/><output>{view}</output></>;
}

describe('Sidebar navigation', () => {
  it('selects each of the four destinations and marks the selected route current', () => {
    render(<SidebarHarness />);

    for (const [label, view] of [['✦ Workbench', 'workbench'], ['⌕ Search vault', 'search'], ['◒ Compass', 'compass'], ['⚙ AI setup', 'ai-setup']] as const) {
      fireEvent.click(screen.getByRole('button', { name: label }));
      expect(screen.getByRole('status')).toHaveTextContent(view);
      expect(screen.getByRole('button', { name: label })).toHaveAttribute('aria-current', 'page');
    }
  });
});
