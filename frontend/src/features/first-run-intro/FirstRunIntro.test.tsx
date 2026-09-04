import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { FirstRunIntro } from './FirstRunIntro';

describe('First-run introduction', () => {
  it('Given a genuine first install, when the welcome surface appears, then it is an OS-like stage rather than a dialog', () => {
    render(<FirstRunIntro onFinish={vi.fn()} />);

    expect(screen.getByRole('main')).toHaveClass('first-run-intro');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'A calmer place for unfinished thoughts.' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Skip intro' })).toBeVisible();
  });

  it('Given the introduction, when the user advances every scene, then Vault selection starts once', async () => {
    const finish = vi.fn().mockResolvedValue(false);
    render(<FirstRunIntro onFinish={finish} />);

    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
    expect(screen.getByRole('heading', { name: 'Your knowledge stays yours.' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
    expect(screen.getByRole('heading', { name: 'From question to working answer.' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Choose Vault' }));

    await waitFor(() => expect(finish).toHaveBeenCalledTimes(1));
  });

  it('Given the introduction, when the user skips it, then the same native setup handoff runs', async () => {
    const finish = vi.fn().mockResolvedValue(false);
    render(<FirstRunIntro onFinish={finish} />);

    fireEvent.click(screen.getByRole('button', { name: 'Skip intro' }));

    await waitFor(() => expect(finish).toHaveBeenCalledTimes(1));
  });

  it('Given the native handoff fails, when the user retries, then recovery remains available', async () => {
    const finish = vi.fn().mockRejectedValueOnce(new Error('picker failed')).mockResolvedValue(false);
    render(<FirstRunIntro onFinish={finish} />);

    fireEvent.click(screen.getByRole('button', { name: 'Skip intro' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('The Vault picker could not be opened.');
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));

    await waitFor(() => expect(finish).toHaveBeenCalledTimes(2));
  });
});
