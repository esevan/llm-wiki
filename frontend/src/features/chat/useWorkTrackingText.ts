import { useSyncExternalStore } from 'react';
import english from '../../../public/i18n/en.json';
import korean from '../../../public/i18n/ko.json';

function subscribe(changed: () => void) {
  const observer = new MutationObserver(changed);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
  return () => observer.disconnect();
}

export function useWorkTrackingText() {
  const ko = useSyncExternalStore(subscribe, () => document.documentElement.lang.startsWith('ko'), () => false);
  const resources: Record<string,string> = ko ? korean : english;
  return (key: string) => resources['work_tracking.'+key] ?? key;
}
