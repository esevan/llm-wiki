import { useEffect, useRef } from 'react';

import './vault-setup.css';

type Phase = 'checking' | 'required' | 'choosing' | 'error';

interface VaultSetupViewProps {
  phase: Phase;
  error?: string;
  onChoose(): void;
  onRetry(): void;
}

const copy = {
  en: {
    eyebrow: 'FIRST-RUN SETUP',
    title: 'Choose where your knowledge lives',
    body: 'Select a folder for your Markdown Vault. LLM Wiki will index and update only the folder you choose.',
    detail: 'Your workflow database stays in the app data folder. Vault documents remain readable Markdown files.',
    choose: 'Choose Vault folder',
    checking: 'Preparing local storage…',
    choosing: 'Waiting for your folder choice…',
    retry: 'Try again',
  },
  ko: {
    eyebrow: '최초 실행 설정',
    title: '지식을 보관할 위치를 선택하세요',
    body: 'Markdown Vault로 사용할 폴더를 선택하세요. LLM Wiki는 사용자가 선택한 폴더만 색인하고 업데이트합니다.',
    detail: '워크플로 데이터베이스는 앱 데이터 폴더에 보관되며, Vault 문서는 읽을 수 있는 Markdown 파일로 유지됩니다.',
    choose: 'Vault 폴더 선택',
    checking: '로컬 저장소 준비 중…',
    choosing: '폴더 선택을 기다리는 중…',
    retry: '다시 시도',
  },
} as const;

export function VaultSetupView({ phase, error, onChoose, onRetry }: VaultSetupViewProps) {
  const dialogRef = useRef<HTMLElement>(null);
  const text = navigator.language.toLowerCase().startsWith('ko') ? copy.ko : copy.en;
  const busy = phase === 'checking' || phase === 'choosing';

  useEffect(() => {
    dialogRef.current?.focus();
  }, []);

  return (
    <div className="vault-setup-backdrop" role="presentation">
      <section
        ref={dialogRef}
        className="vault-setup-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby="vault-setup-title"
        tabIndex={-1}
      >
        <div className="vault-setup-mark" aria-hidden="true">⌂</div>
        <p className="vault-setup-eyebrow">{text.eyebrow}</p>
        <h1 id="vault-setup-title">{text.title}</h1>
        <p>{text.body}</p>
        <p className="vault-setup-detail">{text.detail}</p>
        {error && <p className="vault-setup-error" role="alert">{error}</p>}
        {phase === 'error' ? (
          <button className="primary" type="button" onClick={onRetry}>{text.retry}</button>
        ) : (
          <button className="primary" type="button" disabled={busy} onClick={onChoose} aria-busy={busy}>
            {phase === 'checking' ? text.checking : phase === 'choosing' ? text.choosing : text.choose}
          </button>
        )}
      </section>
    </div>
  );
}
