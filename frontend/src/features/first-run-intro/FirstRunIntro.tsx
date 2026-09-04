import { useEffect, useRef, useState } from 'react';

import './first-run-intro.css';

type FirstRunIntroProps = {
  onFinish: () => Promise<unknown>;
};

const copy = {
  en: {
    skip: 'Skip intro',
    continue: 'Continue',
    start: 'Choose Vault',
    working: 'Opening Vault picker…',
    error: 'The Vault picker could not be opened.',
    retry: 'Try again',
    scenes: [
      {
        title: ['A calmer place for', 'unfinished thoughts.'],
        body: ['Capture fragments now.', 'Shape them when they are ready.'],
      },
      {
        title: ['Your knowledge', 'stays yours.'],
        body: ['A local Markdown Vault,', 'searchable and portable.'],
      },
      {
        title: ['From question to', 'working answer.'],
        body: ['Follow the thread, keep the evidence,', 'and move work forward.'],
      },
    ],
  },
  ko: {
    skip: '소개 건너뛰기',
    continue: '계속',
    start: 'Vault 선택',
    working: 'Vault 선택기 여는 중…',
    error: 'Vault 선택기를 열 수 없습니다.',
    retry: '다시 시도',
    scenes: [
      {
        title: ['정리되지 않은', '생각을 위한', '차분한 공간.'],
        body: ['떠오른 조각을 담고,', '준비됐을 때 다듬으세요.'],
      },
      {
        title: ['지식은 온전히', '사용자에게 남습니다.'],
        body: ['로컬 Markdown Vault로 검색하고,', '어디서든 옮길 수 있어요.'],
      },
      {
        title: ['질문에서', '실제 답까지.'],
        body: ['맥락과 근거를 지키며,', '작업을 앞으로 이어가세요.'],
      },
    ],
  },
} as const;

function SceneArtwork({ scene }: { scene: number }) {
  if (scene === 0) {
    return (
      <div className="intro-art intro-art--capture" aria-hidden="true">
        <span className="intro-fragment intro-fragment--one">idea.md</span>
        <span className="intro-fragment intro-fragment--two">decision</span>
        <span className="intro-fragment intro-fragment--three">question?</span>
        <span className="intro-stack" />
      </div>
    );
  }
  if (scene === 1) {
    return (
      <div className="intro-art intro-art--local" aria-hidden="true">
        <span className="intro-file intro-file--back" />
        <span className="intro-file intro-file--front"><i /><i /><i /></span>
        <span className="intro-search-ring" />
      </div>
    );
  }
  return (
    <div className="intro-art intro-art--workflow" aria-hidden="true">
      <span className="intro-path" />
      <span className="intro-node intro-node--one" />
      <span className="intro-node intro-node--two" />
      <span className="intro-node intro-node--three" />
      <span className="intro-pulse" />
    </div>
  );
}

export function FirstRunIntro({ onFinish }: FirstRunIntroProps) {
  const language = navigator.language.toLowerCase().startsWith('ko') ? 'ko' : 'en';
  const text = copy[language];
  const [scene, setScene] = useState(0);
  const currentScene = text.scenes[scene];
  const [phase, setPhase] = useState<'ready' | 'working' | 'error'>('ready');
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    headingRef.current?.focus();
  }, [scene]);

  useEffect(() => {
    const reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    if (reducedMotion || scene === text.scenes.length - 1) return;
    const timer = window.setTimeout(() => setScene((current) => current + 1), 2600);
    return () => window.clearTimeout(timer);
  }, [scene, text.scenes.length]);

  const finish = async () => {
    if (phase === 'working') return;
    setPhase('working');
    try {
      await onFinish();
    } catch {
      setPhase('error');
    }
  };

  const advance = () => {
    if (scene < text.scenes.length - 1) setScene((current) => current + 1);
    else void finish();
  };

  return (
    <main className="first-run-intro" aria-labelledby="first-run-intro-title">
      <div className="intro-veil" aria-hidden="true" />
      <div className="intro-ambient" aria-hidden="true">
        <span className="intro-orb intro-orb--one" />
        <span className="intro-orb intro-orb--two" />
        <span className="intro-orb intro-orb--three" />
      </div>
      <button className="intro-skip" type="button" onClick={() => void finish()} disabled={phase === 'working'}>
        {text.skip}
      </button>
      <section className="intro-stage" key={scene} aria-live="polite">
        <SceneArtwork scene={scene} />
        <div className="intro-copy">
          <p className="intro-eyebrow">LLM WIKI</p>
          <h1
            id="first-run-intro-title"
            ref={headingRef}
            tabIndex={-1}
            aria-label={currentScene.title.join(' ')}
          >
            {currentScene.title.map((line) => (
              <span key={line} aria-hidden="true">{line}</span>
            ))}
          </h1>
          <p>
            {currentScene.body.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </p>
        </div>
      </section>
      <nav className="intro-controls" aria-label="Introduction progress">
        <div className="intro-progress" aria-label={`${scene + 1} / ${text.scenes.length}`}>
          {text.scenes.map((_, index) => (
            <button
              key={index}
              type="button"
              className={index === scene ? 'active' : ''}
              aria-label={`${index + 1}`}
              aria-current={index === scene ? 'step' : undefined}
              onClick={() => setScene(index)}
              disabled={phase === 'working'}
            />
          ))}
        </div>
        <button className="intro-next" type="button" onClick={advance} disabled={phase === 'working'}>
          {phase === 'working' ? text.working : scene === text.scenes.length - 1 ? text.start : text.continue}
        </button>
        {phase === 'error' && (
          <div className="intro-error" role="alert">
            <span>{text.error}</span>
            <button type="button" onClick={() => void finish()}>{text.retry}</button>
          </div>
        )}
      </nav>
    </main>
  );
}
