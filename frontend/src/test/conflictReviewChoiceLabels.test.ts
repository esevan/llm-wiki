import en from '../../public/i18n/en.json';
import ko from '../../public/i18n/ko.json';
import { expect, it } from 'vitest';

it('CB-034 presents the two requested conflict choices in both locales', () => {
  expect([ko['conflict_review.apply_recommendation'], ko['conflict_review.accept_conflict']]).toEqual(['권장안 적용', '충돌 무시']);
  expect([en['conflict_review.apply_recommendation'], en['conflict_review.accept_conflict']]).toEqual(['Apply recommendation', 'Ignore conflict']);
  expect(ko['conflict_review.rationale_placeholder']).toContain('충돌 무시 시 필수');
  expect(en['conflict_review.rationale_placeholder']).toContain('required when ignoring a conflict');
});
