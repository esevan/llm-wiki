import { useCallback, useEffect, useRef, useState } from 'react';
import { taskClient } from '../../services/taskClient';
import type { TaskCard } from '../../types/taskWorkbench';
import { completedTasksNewestFirst } from '../workbench/completedTasks';
import { useTaskWorkbenchText } from '../workbench/taskWorkbenchText';
import { TaskDetail, type TaskDetailHandle } from '../workbench/TaskDetail';
import { IconButton } from '../../components/IconButton';

export function CompassView({ active }: { active: boolean }) {
  const text = useTaskWorkbenchText();
  const [tab, setTab] = useState<'direction' | 'completed'>('direction');
  const [tasks, setTasks] = useState<TaskCard[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState<string>();
  const detail = useRef<TaskDetailHandle>(null);
  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const snapshot = await taskClient.workbench();
      setTasks(completedTasksNewestFirst(snapshot.categories.flatMap(category => category.items)));
    } catch (error) { setError(String(error instanceof Error ? error.message : error)); }
    finally { setLoading(false); }
  }, []);
  useEffect(() => {
    if (!active || tab !== 'completed') return;
    void load();
    const refresh = () => void load();
    window.addEventListener('llm-wiki:task-workbench-refresh', refresh);
    return () => window.removeEventListener('llm-wiki:task-workbench-refresh', refresh);
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
        <ol className="completed-task-list">{tasks.map(task => <li key={task.id}>
          <button type="button" data-control="compass-completed-open" onClick={() => leaveDetail(() => setSelected(task.id))}>
            {task.title}<small>{task.category}{task.completedAt && ` · ${new Date(task.completedAt).toLocaleDateString(document.documentElement.lang || 'en')}`}</small>
          </button>
        </li>)}</ol>
        {selected && <TaskDetail key={selected} ref={detail} taskId={selected} onClose={() => leaveDetail(() => setSelected(undefined))} onOpenTask={id => leaveDetail(() => setSelected(id))} onChanged={() => { window.dispatchEvent(new Event('llm-wiki:task-workbench-refresh')); }} />}
      </section>}
    </section>
  );
}
