import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { VaultSetupView } from './VaultSetupView';

describe('first-run Vault setup', () => {
  it('Given no configured Vault, when setup appears, then the user chooses the folder explicitly', () => {
    const onChoose = vi.fn();
    render(<VaultSetupView phase="required" onChoose={onChoose} onRetry={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /choose vault folder/i }));

    expect(onChoose).toHaveBeenCalledOnce();
    expect(screen.getByRole('dialog')).toHaveTextContent(/markdown vault/i);
    expect(screen.getByRole('dialog')).toHaveFocus();
  });

  it('Given the native picker is open, then duplicate selection is unavailable', () => {
    render(<VaultSetupView phase="choosing" onChoose={vi.fn()} onRetry={vi.fn()} />);

    expect(screen.getByRole('button')).toBeDisabled();
    expect(screen.getByRole('button')).toHaveAttribute('aria-busy', 'true');
  });
});
