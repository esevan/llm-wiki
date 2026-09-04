import { describe, expect, it } from 'vitest';
import { formatSystemTime, parseNativeTimestamp } from './systemTime';

describe('native timestamp presentation', () => {
  it('Given a timezone-free SQLite UTC timestamp, when parsed, then it represents UTC', () => {
    expect(parseNativeTimestamp('2026-01-15 18:30:00').toISOString()).toBe('2026-01-15T18:30:00.000Z');
  });

  it('Given a UTC timestamp, when displayed in a system timezone, then local wall time is shown', () => {
    expect(formatSystemTime('2026-01-15 18:30:00', 'en-US', {
      dateStyle: 'medium',
      timeStyle: 'short',
      timeZone: 'America/New_York',
    })).toBe('Jan 15, 2026, 1:30 PM');
  });

  it('Given an invalid stored timestamp, when displayed, then the original value remains visible', () => {
    expect(formatSystemTime('not-a-date', 'en-US')).toBe('not-a-date');
  });
});
