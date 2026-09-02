import { IconButton } from '../../components/IconButton';

export function SearchView({ active }: { active: boolean }) {
  return (
    <section id="search" className={`view${active ? ' active' : ''}`}>
      <header className="top"><div><div className="eyebrow">Your connected notes</div><h1>Find context</h1></div></header>
      <form className="searchbox" id="search-form">
        <input id="query" placeholder="Search paths, tags, headings, or words…" />
        <label className="pill"><input id="semantic" type="checkbox" /> Semantic</label>
        <IconButton kind="primary" label="Search vault">⌕</IconButton>
      </form>
      <section id="results" className="results" />
    </section>
  );
}
