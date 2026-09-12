import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from './App';

describe('React shell behavior', () => {
  it('Given the app launches, when React renders, then each current workspace surface is available', () => {
    render(<App />);

    expect(screen.getByRole('heading', { name: 'Your workbench' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Find context' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Your Compass' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'AI setup' })).toBeInTheDocument();
  });

  it('Given an empty workbench, when the shell renders, then direct Capture and Task entry remain available', () => {
    render(<App />);

    expect(screen.getByLabelText('Workbench entry')).toBeRequired();
    expect(screen.getByRole('radio', { name: 'Capture a thought' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Register as a Task' })).not.toBeChecked();
  });

  it('Given AI setup, when it renders, then secrets and model routing remain separated', () => {
    render(<App />);

    expect(document.querySelector<HTMLInputElement>('#provider-key')?.type).toBe('password');
    expect(document.querySelectorAll('[data-advanced-task]')).toHaveLength(13);
    expect(screen.getByText(/never in the vault or app database/i)).toBeInTheDocument();
  });

  it('Given the Workbench is active, when Search is selected, then React owns the visible route', () => {
    render(<App />);

    const searchNavigation = document.querySelector<HTMLButtonElement>('[data-view="search"]');
    expect(searchNavigation).not.toBeNull();
    fireEvent.click(searchNavigation!);

    expect(document.querySelector('#search')).toHaveClass('active');
    expect(document.querySelector('#workbench')).not.toHaveClass('active');
    expect(searchNavigation).toHaveClass('active');
  });
});
