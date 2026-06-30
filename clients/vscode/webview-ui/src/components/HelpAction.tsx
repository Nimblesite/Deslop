import type { ComponentChildren } from "preact";

import { HelpBubble, type HelpTopic } from "./HelpBubble";

export function HelpAction({ topic, children }: { topic: HelpTopic; children: ComponentChildren }) {
  return (
    <span class="with-help">
      {children}
      <HelpBubble topic={topic} />
    </span>
  );
}
