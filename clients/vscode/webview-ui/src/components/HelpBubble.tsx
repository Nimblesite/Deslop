import type { ComponentChildren, JSX } from "preact";

const DOCS_BASE = "https://deslop.live/docs/vscode-cluster-panel/";

/** Every helped element of the cluster panel. */
export type HelpTopic =
  | "cluster-id"
  | "ai-match"
  | "rank"
  | "mass"
  | "occurrence-count"
  | "canonical"
  | "occurrences"
  | "occurrence-location"
  | "hidden-occurrence"
  | "open-action"
  | "compare-action"
  | "cluster-navigation"
  | "keyboard-shortcuts";

const PANEL_HELP: Record<HelpTopic, string> = {
  "cluster-id": "Stable identifier for this duplicate-code cluster.",
  "ai-match": "The embedding pass found a semantic match, not only a syntactic one.",
  rank: "Worst-first position of this cluster in the current report.",
  mass: "This cluster's duplicated mass — the worst-first ranking metric.",
  "occurrence-count": "Number of editor locations in this cluster.",
  canonical: "First occurrence of the cluster — its canonical extent. Compare never uses it implicitly; pair comparison is explicit two-endpoint selection only.",
  occurrences: "The concrete locations where this cluster appears.",
  "occurrence-location": "File, line, and column that Open will navigate to.",
  "hidden-occurrence": "This occurrence matched report_hide configuration.",
  "open-action": "Open selects the clone range in the editor.",
  "compare-action": "Compare opens a diff between the two occurrences you selected.",
  "cluster-navigation": "Move between clusters without leaving this panel.",
  "keyboard-shortcuts": "Keyboard actions available while focus is in the panel.",
};

// [FUSED-PAIR-SIGNALS] The cluster panel renders cluster facts, never
// signal bars: the measured axes describe one pair of occurrences and have
// nothing to do with the cluster ([FUSED-CONTENT-GATE]). No signal help copy
// lives here because no signal is rendered here.
const HELP_COPY: Record<HelpTopic, string> = PANEL_HELP;

interface HelpBubbleProps {
  topic: HelpTopic;
}

interface HelpedTextProps extends HelpBubbleProps {
  children: ComponentChildren;
  className?: string;
  style?: JSX.CSSProperties;
  title?: string;
}

export function helpCopy(topic: HelpTopic): string {
  return HELP_COPY[topic];
}

export function docsUrl(topic: HelpTopic): string {
  return `${DOCS_BASE}#${topic}`;
}

export function HelpBubble({ topic }: HelpBubbleProps) {
  const title = `${helpCopy(topic)} More details: ${docsUrl(topic)}`;
  return (
    <a
      class="help-bubble"
      data-doc-topic={topic}
      href={docsUrl(topic)}
      target="_blank"
      rel="noopener noreferrer"
      title={title}
      aria-label={title}
    >
      ?
    </a>
  );
}

export function DocTextLink({
  topic,
  children,
  className,
  style,
  title,
}: HelpedTextProps) {
  return (
    <a
      class={mergeClass("doc-link", className)}
      href={docsUrl(topic)}
      target="_blank"
      rel="noopener noreferrer"
      style={style}
      title={title ?? helpCopy(topic)}
    >
      {children}
    </a>
  );
}

export function HelpedText({
  topic,
  children,
  className,
  style,
  title,
}: HelpedTextProps) {
  return (
    <span class={mergeClass("with-help", className)} style={style} title={title ?? helpCopy(topic)}>
      <span>{children}</span>
      <HelpBubble topic={topic} />
    </span>
  );
}

function mergeClass(base: string, extra?: string): string {
  return extra ? `${base} ${extra}` : base;
}
