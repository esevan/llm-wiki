import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { chooseVault, completeFirstRunIntro, getVaultSetupStatus } from './vaultSetupClient';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('Vault setup client', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    window.__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete window.__TAURI_INTERNALS__;
  });

  it('queries setup without exposing a filesystem path input', async () => {
    vi.mocked(invoke).mockResolvedValue({ required: true, path: null, introRequired: true });

    await expect(getVaultSetupStatus()).resolves.toEqual({
      required: true,
      path: null,
      introRequired: true,
    });

    expect(invoke).toHaveBeenCalledWith('vault_setup_status');
  });

  it('opens the native picker through a dedicated command', async () => {
    vi.mocked(invoke).mockResolvedValue(false);

    await expect(chooseVault()).resolves.toBe(false);

    expect(invoke).toHaveBeenCalledWith('choose_vault');
  });

  it('completes the one-time intro before opening the native picker', async () => {
    vi.mocked(invoke).mockResolvedValue(false);

    await expect(completeFirstRunIntro()).resolves.toBe(false);

    expect(invoke).toHaveBeenCalledWith('complete_first_run_intro');
  });
});
