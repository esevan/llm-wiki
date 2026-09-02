import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from './App';

describe('React shell behavior', () => {
  it('Given the app launches, when React renders, then every legacy entry point has an equivalent screen', () => {
    render(<App />);

    expect(screen.getByRole('heading', { name: 'What’s on your mind?' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Find context' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Your Compass' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'AI setup' })).toBeInTheDocument();
  });

  it('Given an empty workbench, when the shell renders, then capture remains lightweight and workflow regions are ready', () => {
    render(<App />);

    expect(screen.getByPlaceholderText('e.g. I keep losing track of decisions…')).toBeRequired();
    expect(document.querySelector('#board')).toBeEmptyDOMElement();
    expect(document.querySelector('#recent-archive')).toBeEmptyDOMElement();
    expect(document.querySelector('#completed-solutions')).toBeEmptyDOMElement();
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
