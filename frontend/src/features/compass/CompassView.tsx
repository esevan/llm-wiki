import { useCallback, useEffect, useRef, useState } from 'react';
import { taskClient } from '../../services/taskClient';
import type { TaskCard } from '../../types/taskWorkbench';
import { completedTasksNewestFirst } from '../workbench/completedTasks';
import { useTaskWorkbenchText } from '../workbench/taskWorkbenchText';
import { TaskDetail, type TaskDetailHandle } from '../workbench/TaskDetail';
import './compass-publication.css';
import { IconButton } from '../../components/IconButton';

export function CompassView({ active }: { active: boolean }) {
  const text = useTaskWorkbenchText();
  const [tab, setTab] = useState<'direction' | 'completed'>('direction');
  const [tasks, setTasks] = useState<TaskCard[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [confirmation, setConfirmation] = useState<TaskCard>();
  const [savingChoice, setSavingChoice] = useState(false);
  const [choiceError, setChoiceError] = useState("");
  const choiceInFlight = useRef(false);
  const choiceTrigger = useRef<HTMLButtonElement | null>(null);
  const confirmDialog = useRef<HTMLDialogElement | null>(null);
  const tags = text.publicationTags;
  const closeConfirmation = () => { setConfirmation(undefined); setChoiceError(""); choiceTrigger.current?.focus(); };
  useEffect(() => { if (!active) { setConfirmation(undefined); setChoiceError(""); } }, [active]);
  useEffect(() => {
    if (confirmation && active) confirmDialog.current?.showModal?.();
  }, [confirmation, active]);
  const saveChoice = async (task: TaskCard, wontPublish: boolean) => {
    if (choiceInFlight.current) return;
    choiceInFlight.current = true; setSavingChoice(true); setChoiceError("");
    try {
      const status = await taskClient.publicationChoice(task.id, task.taskRevision, task.knowledgePublication?.choiceRevision ?? 0, wontPublish);
      setTasks(current => current.map(row => row.id === task.id ? { ...row, knowledgePublication: status } : row));
      closeConfirmation();
      window.dispatchEvent(new Event('llm-wiki:task-workbench-refresh'));
    } catch { setChoiceError(tags.failed); }
    finally { choiceInFlight.current = false; setSavingChoice(false); }
  };
  const [selected, setSelected] = useState<string>();
  const detail = useRef<TaskDetailHandle>(null);
  const detailTrigger = useRef<HTMLElement | null>(null);
  const closeDetail = () => {
    setSelected(undefined);
    requestAnimationFrame(() => {
      const trigger = detailTrigger.current;
      if (trigger?.isConnected && !trigger.closest('.view:not(.active), [hidden]')) trigger.focus({ preventScroll: true });
    });
  };
  const openDetail = (id: string, trigger?: HTMLElement) => {
    if (selected === id) return;
    leaveDetail(() => {
      if (trigger) detailTrigger.current = trigger;
      setSelected(id);
    });
  };
  const loadSequence = useRef(0);
  const load = useCallback(async () => {
    const sequence = ++loadSequence.current;
    setLoading(true);
    setError('');
    try {
      const snapshot = await taskClient.workbench();
      if (sequence === loadSequence.current) setTasks(completedTasksNewestFirst(snapshot.categories.flatMap(category => category.items)));
    } catch (error) { if (sequence === loadSequence.current) setError(String(error instanceof Error ? error.message : error)); }
    finally { if (sequence === loadSequence.current) setLoading(false); }
  }, []);
  useEffect(() => {
    if (!active || tab !== 'completed') return;
    void load();
    const refresh = () => void load();
    window.addEventListener('llm-wiki:task-workbench-refresh', refresh);
    // Job completion and revision chat can change publication facts outside this view.
    const timer = window.setInterval(refresh, 5000);
    return () => { window.clearInterval(timer); window.removeEventListener('llm-wiki:task-workbench-refresh', refresh); };
  }, [active, tab, load]);
  const leaveDetail = (next: () => void) => detail.current ? detail.current.requestLeave(next) : next();
  return (
    <section id="compass" className={`view${active ? ' active' : ''}`}>
      <header className="top"><div><div className="eyebrow">Direction, not busyness</div><h1>Your Compass</h1></div></header>
      <nav className="compass-tabs" aria-label="Compass">
        <button type="button" data-control="compass-direction-tab" aria-pressed={tab === 'direction'} onClick={() => leaveDetail(() => { setSelected(undefined); setTab('direction'); })}>{text.directionTab}</button>
        <button type="button" data-control="compass-completed-tab" aria-pressed={tab === 'completed'} onClick={() => setTab('completed')}>{text.completedList}</button>
      </nav>
      <div hidden={tab !== 'direction'}>
      <section className="quick">
        <h2>Choose a direction.</h2>
        <p>Goals make the work you approve measurable without turning activity into achievement.</p>
        <form className="capture" id="goal-form">
          <input id="goal-title" data-control="compass-goal-title" aria-label="Direction goal" placeholder="e.g. Make decisions easier to reuse" required />
          <IconButton kind="primary" data-control="compass-goal-save" label="Add goal" labelVisible>+</IconButton>
        </form>
      </section>
      <section id="dashboard" className="results" />
      </div>
      {tab === 'completed' && <section aria-label={text.completedList}>
        <h2>{text.completedList}</h2>
        {loading && <p role="status">{text.loading}</p>}
        {error && <div role="alert">{error}<button type="button" data-control="compass-completed-retry" onClick={() => void load()}>{text.retry}</button></div>}
        {!loading && !error && !tasks.length && <p>{text.noCompleted}</p>}
        {choiceError && !confirmation && <p role="alert">{choiceError}</p>}
        <ol className="completed-task-list compass-completed-list">{tasks.map(task => {
          const publication = task.knowledgePublication;
          const pending = publication?.proposal && publication.proposal !== 'none';
          return <li key={task.id}>
            <button className="compass-task-open" type="button" data-control="compass-completed-open" onClick={event => openDetail(task.id, event.currentTarget)}>
              {task.title}<small>{task.category}{task.completedAt && ` · ${new Date(task.completedAt).toLocaleDateString(document.documentElement.lang || 'en')}`}</small>
            </button>
            <div className="compass-publication-crumbs" aria-label={text.knowledgeReview.publicationState}>
              {publication?.published && <span className="compass-publication-chip is-published">{tags.published}</span>}
              {!publication?.published && !pending && <span className="compass-publication-chip">{tags.unpublished}</span>}
              {pending && <><span className="compass-publication-chip is-pending">{tags.pending}</span><span className="compass-publication-detail">{publication?.proposal === 'generating' ? tags.generating : tags.awaiting}</span></>}
              {publication?.wontPublish && <><span className="compass-publication-chip is-optout">{tags.optOut}</span><button className="compass-publication-action" type="button" disabled={savingChoice} data-control="compass-publication-undo" onClick={() => void saveChoice(task, false)}>{tags.undo}</button></>}
              {!publication?.wontPublish && !publication?.published && !pending && <button className="compass-publication-action" type="button" data-control="compass-publication-optout" disabled={savingChoice} onClick={event => { choiceTrigger.current = event.currentTarget; setChoiceError(""); setConfirmation(task); }}>{tags.choose}</button>}
            </div>
          </li>;
        })}</ol>
        {confirmation && active && <dialog ref={confirmDialog} className="compass-publication-confirm" aria-labelledby="compass-choice-title" onCancel={event => { event.preventDefault(); if (!choiceInFlight.current) closeConfirmation(); }}>
          <h2 id="compass-choice-title">{tags.confirmTitle}</h2>
          <p>{tags.confirmBody}</p><div className="compass-confirm-task">{confirmation.title}</div>
          <div className="compass-publication-crumbs"><span className="compass-publication-chip">{tags.unpublished}</span><span className="compass-publication-chip is-optout">{tags.optOut}</span></div>
          {choiceError && <p role="alert">{choiceError}</p>}
          <div className="compass-confirm-actions"><button type="button" data-control="compass-publication-cancel" disabled={savingChoice} onClick={closeConfirmation}>{tags.cancel}</button><button type="button" className="primary" data-control="compass-publication-confirm" disabled={savingChoice} onClick={() => void saveChoice(confirmation, true)}>{savingChoice ? tags.saving : tags.confirm}</button></div>
        </dialog>}
        {selected && <TaskDetail key={selected} ref={detail} taskId={selected} initialTab="work"
          onPrepareKnowledgeArchive={taskClient.prepareKnowledgeArchive}
          onOrganizeKnowledgeArchive={taskClient.organizeKnowledgeArchive}
          onPublishKnowledgeArchive={taskClient.publishKnowledgeArchive}
          onKnowledgeArchiveStatus={taskClient.knowledgeArchiveStatus}
          onRetryKnowledgeArchive={taskClient.retryKnowledgeArchive}
          onRecoverKnowledgeArchive={taskClient.recoverKnowledgeArchive}
          onClose={closeDetail} onOpenTask={id => openDetail(id)} onChanged={() => { window.dispatchEvent(new Event('llm-wiki:task-workbench-refresh')); }} />}
      </section>}
    </section>
  );
}
