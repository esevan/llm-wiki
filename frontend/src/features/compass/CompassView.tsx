import { IconButton } from '../../components/IconButton';

export function CompassView({ active }: { active: boolean }) {
  return (
    <section id="compass" className={`view${active ? ' active' : ''}`}>
      <header className="top"><div><div className="eyebrow">Direction, not busyness</div><h1>Your Compass</h1></div></header>
      <section className="quick">
        <h2>Choose a direction.</h2>
        <p>Goals make the work you approve measurable without turning activity into achievement.</p>
        <form className="capture" id="goal-form">
          <input id="goal-title" placeholder="e.g. Make decisions easier to reuse" required />
          <IconButton kind="primary" label="Add goal">+</IconButton>
        </form>
      </section>
      <section id="dashboard" className="results" />
    </section>
  );
}
