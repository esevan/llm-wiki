import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type HTMLAttributes,
  type ReactNode,
} from "react";

export type ContentTransitionCause =
  | "preview_adopted"
  | "automatic_apply"
  | "refinement_apply"
  | "version_restore";

type ElementName = "div" | "h1" | "h2" | "h3" | "h4" | "p" | "span";
type ContentRevision = number | string;

interface ContentRevisionTransitionProps extends HTMLAttributes<HTMLElement> {
  as?: ElementName;
  children: string;
  cause?: ContentTransitionCause;
  entityKey: string;
  historical?: boolean;
  reducedMotion?: boolean;
  renderContent?: (content: string) => ReactNode;
  revision: ContentRevision;
  variant: "title" | "body";
}

interface Snapshot {
  content: string;
  entityKey: string;
  revision: ContentRevision;
}

interface ActiveTransition {
  content: string;
  previousContent?: string;
  revision: ContentRevision;
}

const supportedCauses = new Set<ContentTransitionCause>([
  "preview_adopted",
  "automatic_apply",
  "refinement_apply",
  "version_restore",
]);

function useReducedMotion(override: boolean | undefined) {
  const [systemPreference, setSystemPreference] = useState(() =>
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
      : false,
  );

  useEffect(() => {
    if (override !== undefined || typeof window.matchMedia !== "function") return;
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setSystemPreference(query.matches);
    update();
    query.addEventListener?.("change", update);
    return () => query.removeEventListener?.("change", update);
  }, [override]);

  return override ?? systemPreference;
}

export function ContentRevisionTransition({
  as = "span",
  children,
  cause,
  className = "",
  entityKey,
  historical = false,
  reducedMotion,
  renderContent = content => content,
  revision,
  variant,
  ...attributes
}: ContentRevisionTransitionProps) {
  const Element = as;
  const prefersReducedMotion = useReducedMotion(reducedMotion);
  const previous = useRef<Snapshot | undefined>(undefined);
  const currentLayer = useRef<HTMLSpanElement | null>(null);
  const [active, setActive] = useState<ActiveTransition>();

  useLayoutEffect(() => {
    const prior = previous.current;
    const shouldAnimate = Boolean(
        prior &&
        prior.entityKey === entityKey &&
        prior.revision !== revision &&
        prior.content !== children &&
        cause &&
        supportedCauses.has(cause) &&
        !historical &&
        !prefersReducedMotion,
    );

    previous.current = { content: children, entityKey, revision };
    if (!shouldAnimate) {
      setActive(undefined);
      return;
    }
    setActive({
      content: children,
      previousContent: variant === "title" ? prior?.content : undefined,
      revision,
    });
  }, [cause, children, entityKey, historical, prefersReducedMotion, revision, variant]);

  useEffect(() => {
    const layer = currentLayer.current;
    if (!layer || !active) return;
    const expectedContent = active.content;
    const expectedRevision = active.revision;
    const finishCurrentTransition = () =>
      setActive((current) =>
        current?.revision === expectedRevision && current.content === expectedContent
          ? undefined
          : current,
      );
    layer.addEventListener("animationcancel", finishCurrentTransition);
    return () => layer.removeEventListener("animationcancel", finishCurrentTransition);
  }, [active]);

  const rootClassName = [
    "content-revision-transition",
    `content-revision-transition-${variant}`,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  if (!active || active.revision !== revision || active.content !== children) {
    return (
      <Element className={rootClassName} {...attributes}>
        {renderContent(children)}
      </Element>
    );
  }

  return (
    <Element
      className={rootClassName}
      data-transitioning={variant}
      {...attributes}
    >
      <span className="sr-only" data-accessible-content="true">
        {children}
      </span>
      {active.previousContent !== undefined && (
        <span
          key={`previous:${String(active.revision)}`}
          aria-hidden="true"
          data-content-layer="previous"
        >
          {renderContent(active.previousContent)}
        </span>
      )}
      <span
        key={`current:${String(active.revision)}`}
        ref={currentLayer}
        aria-hidden="true"
        data-content-layer="current"
        onAnimationEnd={(event) => {
          if (event.currentTarget !== event.target) return;
          const expectedContent = active.content;
          const expectedRevision = active.revision;
          setActive((current) =>
            current?.revision === expectedRevision && current.content === expectedContent
              ? undefined
              : current,
          );
        }}
      >
        {renderContent(children)}
      </span>
    </Element>
  );
}
