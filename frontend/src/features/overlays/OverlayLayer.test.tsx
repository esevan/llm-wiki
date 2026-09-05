import { fireEvent, render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { OverlayLayer } from './OverlayLayer';

describe.each(['queue', 'alert'])('CB-031 %s FAB dismissal', name => {
  function setup() {
    const view = render(<OverlayLayer />);
    const panel = document.getElementById(`${name}-panel`)!;
    const toggle = document.getElementById(`${name}-toggle`)!;
    panel.hidden = false;
    toggle.setAttribute('aria-expanded', 'true');
    return { ...view, panel, toggle };
  }
  it('closes on an outside click without consuming the underlying action', () => {
    const { panel, toggle } = setup();
    const outside = document.createElement('button');
    const clicked = vi.fn();
    outside.onclick = clicked;
    document.body.append(outside);
    fireEvent.click(outside);
    expect(panel.hidden).toBe(true);
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(clicked).toHaveBeenCalledOnce();
    outside.remove();
  });
  it('preserves panel and trigger clicks, including nested targets', () => {
    const { panel, toggle } = setup();
    const action = document.createElement('button');
    const clicked = vi.fn();
    action.onclick = clicked;
    panel.append(action);
    fireEvent.click(action);
    expect(clicked).toHaveBeenCalledOnce();
    expect(panel.hidden).toBe(false);
    const icon = document.createElement('span');
    toggle.append(icon);
    fireEvent.click(icon);
    expect(panel.hidden).toBe(false);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
  });
  it('dismisses when the other FAB is clicked', () => {
    const { panel, toggle } = setup();
    fireEvent.click(document.getElementById(`${name === 'queue' ? 'alert' : 'queue'}-toggle`)!);
    expect(panel.hidden).toBe(true);
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
  });
  it('removes its document listener on unmount', () => {
    const { unmount, panel } = setup();
    unmount();
    document.body.append(panel);
    fireEvent.click(document.body);
    expect(panel.hidden).toBe(false);
    panel.remove();
  });
});
