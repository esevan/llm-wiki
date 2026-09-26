import "../features/workbench/reference-aware-workbench.css";
import { useState } from "react";
import type {
  PreviewComparison,
  WorkPreviewSummary,
} from "../types/taskWorkbench";
import { useReferenceWorkbenchText } from "../features/workbench/referenceWorkbenchText";

export interface DraftVersionControlsProps {
  versions: WorkPreviewSummary[];
  selected: number;
  currentVersion?: number;
  comparison?: PreviewComparison;
  onSelect: (version: number) => void;
  onCompare: (left: number, right: number) => void;
  onRestore: (version: number) => void;
  restoreDisabled?: boolean;
  disabled?: boolean;
}
const display = (value: unknown): string => {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(display).join("\n");
  if (value && typeof value === "object" && "text" in value)
    return String(value.text);
  return value == null ? "" : String(value);
};
export function DraftVersionControls({
  versions,
  selected,
  currentVersion,
  comparison,
  onSelect,
  onCompare,
  onRestore,
  restoreDisabled,
  disabled,
}: DraftVersionControlsProps) {
  const text = useReferenceWorkbenchText();
  const current =
    currentVersion ??
    versions.find((version) => version.current)?.version ??
    versions.at(-1)?.version;
  const [left, setLeft] = useState<number>();
  const [right, setRight] = useState<number>();
  const earlier = left ?? versions[0]?.version;
  const later = right ?? selected;
  const options = versions.map((version) => (
    <option key={version.version} value={version.version}>
      {version.version} · {text[version.derivationKind]}
      {version.version === current ? ` · ${text.current}` : ""}
    </option>
  ));
  return (
    <section className="draft-version-controls" aria-label={text.versions}>
      <label>
        {text.version}
        <select
          data-control="draft-version-select"
          value={selected}
          disabled={disabled}
          onChange={(event) => onSelect(Number(event.target.value))}
        >
          {options}
        </select>
      </label>
      {selected !== current && (
        <button
          data-control="draft-version-restore"
          type="button"
          disabled={disabled || restoreDisabled}
          onClick={() => onRestore(selected)}
        >
          {text.restore}
        </button>
      )}
      {versions.length > 1 && (
        <div className="draft-version-selectors">
          <label>
            {text.left}
            <select
              data-control="draft-version-earlier"
              value={earlier}
              onChange={(event) => setLeft(Number(event.target.value))}
            >
              {options}
            </select>
          </label>
          <label>
            {text.right}
            <select
              data-control="draft-version-later"
              value={later}
              onChange={(event) => setRight(Number(event.target.value))}
            >
              {options}
            </select>
          </label>
          <button
            data-control="draft-version-compare"
            type="button"
            disabled={disabled || earlier === later}
            onClick={() => onCompare(earlier, later)}
          >
            {text.compare}
          </button>
        </div>
      )}
      {comparison && (
        <div
          className="draft-version-comparison"
          role="region"
          aria-label={text.comparison}
        >
          {comparison.fields.map((field) => (
            <article key={field.key}>
              <header>
                <strong>{text[field.key]}</strong>
                <span>
                  {field.state === "changed"
                    ? text.changedField
                    : text[field.state]}
                </span>
              </header>
              <div>
                <pre aria-label={`${text.left} ${comparison.left.version}`}>
                  {display(comparison.left.fields[field.key])}
                </pre>
                <pre aria-label={`${text.right} ${comparison.right.version}`}>
                  {display(comparison.right.fields[field.key])}
                </pre>
              </div>
            </article>
          ))}
          <article>
            <strong>{text.references}</strong>
            <div>
              <pre>
                {comparison.left.references
                  .map(
                    (reference) =>
                      `${reference.title} · ${reference.section ?? ""}\n${reference.documentVersion}`,
                  )
                  .join("\n\n")}
              </pre>
              <pre>
                {comparison.right.references
                  .map(
                    (reference) =>
                      `${reference.title} · ${reference.section ?? ""}\n${reference.documentVersion}`,
                  )
                  .join("\n\n")}
              </pre>
            </div>
          </article>
        </div>
      )}
    </section>
  );
}
