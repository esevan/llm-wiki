import { IconButton } from '../../components/IconButton';

export function WorkbenchView({ active }: { active: boolean }) {
  return (
    <section id="workbench" className={`view${active ? ' active' : ''}`}>
      <header className="top">
        <div><div className="eyebrow">Your thinking space</div><h1>What’s on your mind?</h1></div>
        <div className="status">LOCAL • YOUR VAULT</div>
      </header>
      <section className="quick">
        <h2>Start with a Capture.</h2>
        <p>Capture a thought first. Then AI helps turn it into a reviewable Problem.</p>
        <form className="capture" id="capture">
          <input id="capture-text" placeholder="e.g. I keep losing track of decisions…" required />
          <IconButton kind="primary" label="Save Capture">+</IconButton>
        </form>
      </section>
      <div className="board-head">
        <h2>Your workbench</h2>
        <span id="organize-status">Keep attention on what matters now.</span>
        <IconButton id="organize" kind="tiny hot" label="Organize workbench">✦</IconButton>
        <IconButton id="flow-toggle" label="Show flow">⤳</IconButton>
      </div>
      <section id="flow-view" className="flow-view" hidden />
      <section className="board" id="board" />
      <section className="workbench-context">
        <article className="context-panel">
          <div className="panel-head">
            <div><small>SEARCH VAULT</small><h2>Recently archived</h2></div>
            <IconButton id="archive-more" label="Show more archived documents">⋯</IconButton>
          </div>
          <div id="recent-archive" className="context-list" />
        </article>
        <article className="context-panel">
          <div className="panel-head"><div><small>COMPLETED SOLUTIONS</small><h2>Reusable work</h2></div></div>
          <div id="completed-solutions" className="context-list" />
        </article>
      </section>
    </section>
  );
}
