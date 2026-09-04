const HAS_TIME_ZONE = /[zZ]$|[+-]\d\d:?\d\d$/;

export const parseNativeTimestamp = (value: string): Date => {
  const raw = value.trim();
  const timestamp = HAS_TIME_ZONE.test(raw) ? raw : `${raw.replace(' ', 'T')}Z`;
  return new Date(timestamp);
};

export const formatSystemTime = (
  value: string,
  locale: string,
  options: Intl.DateTimeFormatOptions = { dateStyle: 'medium', timeStyle: 'short' },
): string => {
  const raw = value.trim();
  if (!raw) return '';
  const date = parseNativeTimestamp(raw);
  return Number.isNaN(date.valueOf()) ? raw : new Intl.DateTimeFormat(locale, options).format(date);
};
