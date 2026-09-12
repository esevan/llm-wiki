import { render, screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { SearchView } from './SearchView';

it('names the query from the localized search heading instead of its placeholder', () => {
  render(<SearchView active />);
  const query = screen.getByRole('textbox', { name: 'Find context' });
  expect(query).toHaveAttribute('aria-labelledby', 'search-heading');
  expect(query).toHaveAttribute('data-i18n-placeholder', 'search.placeholder');
});
