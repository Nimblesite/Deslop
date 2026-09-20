import type { ComponentChildren, JSX } from "preact";
import { useId, useState } from "preact/hooks";
import MarkdownIt from "markdown-it";

const DOCS_BASE = "https://deslop.live/docs/vscode-cluster-panel/";
const HELP_WIDTH = 340;
const VIEWPORT_MARGIN = 12;
const HELP_GAP = 0;
const markdown = new MarkdownIt({ html: false, linkify: false });

/** Every helped element of the cluster panel. */
export type HelpTopic =
  | "cluster-id"
  | "clone-kind"
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
  "cluster-id": "Stable identifier for this finding.",
  "clone-kind": "**Clone categories**\n\n- **Identical:** the same code.\n- **Nearly identical:** small changes, such as names or values.\n- **Similar:** related code with more substantial changes.\n- **Same shape, different content:** informational; does not count as duplication.\n\nColour identifies the category. Compare source ranges before deciding what to share.",
  "ai-match": "The embedding pass found a semantic match, not only a syntactic one.",
  rank: "Worst-first position of this cluster in the current report.",
  mass: "**Duplicated mass**\n\nThis cluster's duplicated mass is the engine's measure of repeated code. Larger values appear first so you can start with the biggest duplication.\n\nThis is **not a line count or a percentage**. The panel displays the engine's value without recalculating it.",
  "occurrence-count": "Number of editor locations in this cluster.",
  canonical: "**Canonical occurrence**\n\nThe first occurrence chosen by the engine. **Compare To Canonical** places it on the left and the clicked occurrence on the right.\n\nIt is a comparison reference, not a recommendation to keep that implementation. It cannot compare with itself.",
  occurrences: "**Open and compare code**\n\nClick a **file link** to open and select its code range.\n\n- **Compare To Canonical** compares that row with the canonical occurrence.\n- **Select for Compare** chooses the left side of an arbitrary pair.\n- **Compare with Selected** opens the clicked row on the right.\n- **Clear Selection** cancels the selection.\n\nTap one row, then another, to compare those two as a shortcut. Selection clears after comparison or when changing clusters.",
  "occurrence-location": "File, line, and column that Open will navigate to.",
  "hidden-occurrence": "This occurrence matched report_hide configuration.",
  "open-action": "Open selects the clone range in the editor.",
  "compare-action": "**Compare To Canonical**\n\nCompare opens a diff between this occurrence and the canonical occurrence in one click. To choose a different left side, use **Select for Compare**, then **Compare with Selected** on another occurrence.",
  "cluster-navigation": "Move between clusters without leaving this panel.",
  "keyboard-shortcuts": "**Keyboard shortcuts**\n\n- `j` / `k`: next / previous occurrence.\n- `n` / `p`: next / previous cluster.\n- `Enter`: open the focused occurrence.\n- `?`: toggle detailed keyboard help.\n\nUse **Tab** to focus buttons and help links. Shortcuts do not intercept button or input actions.",
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
  const id = useId();
  const [position, setPosition] = useState<JSX.CSSProperties | null>(null);
  const show = (event: Event) => setPosition(helpPosition(event.currentTarget as HTMLElement));
  return <span class="help-anchor" onMouseEnter={show} onMouseLeave={() => setPosition(null)}
    onFocusIn={show} onFocusOut={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setPosition(null); }}
    onKeyDown={(event) => { if (event.key === "Escape") setPosition(null); }}>
    <a class="help-bubble" data-doc-topic={topic} href={docsUrl(topic)} target="_blank"
      rel="noopener noreferrer" title="" aria-label={`Help: ${topic}`} aria-describedby={position ? id : undefined}>?</a>
    {position ? <span id={id} role="tooltip" class="help-popover" style={position}>
      <span dangerouslySetInnerHTML={{ __html: markdown.render(helpCopy(topic)) }} />
      <a href={docsUrl(topic)} target="_blank" rel="noopener noreferrer">More details</a>
    </span> : null}
  </span>;
}

function helpPosition(anchor: HTMLElement): JSX.CSSProperties {
  const rect = anchor.getBoundingClientRect();
  const width = Math.min(HELP_WIDTH, window.innerWidth - VIEWPORT_MARGIN * 2);
  const left = Math.max(VIEWPORT_MARGIN, Math.min(rect.left, window.innerWidth - width - VIEWPORT_MARGIN));
  const vertical = rect.bottom < window.innerHeight / 2
    ? { top: rect.bottom + HELP_GAP, maxHeight: window.innerHeight - rect.bottom - VIEWPORT_MARGIN - HELP_GAP }
    : { bottom: window.innerHeight - rect.top + HELP_GAP, maxHeight: rect.top - VIEWPORT_MARGIN - HELP_GAP };
  return { ...vertical, left, width };
}

export function DocTextLink({
  topic, children, className, style, title,
}: HelpedTextProps) {
  return (
    <a
      class={mergeClass("doc-link", className)}
      href={docsUrl(topic)}
      target="_blank"
      rel="noopener noreferrer"
      style={style}
      title={title ?? `Read about ${topic}`}
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
    <span class={mergeClass("with-help", className)} style={style} title={title}>
      <span>{children}</span>
      <HelpBubble topic={topic} />
    </span>
  );
}

function mergeClass(base: string, extra?: string): string {
  return extra ? `${base} ${extra}` : base;
}
