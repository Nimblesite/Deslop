import { expect, test } from "@playwright/test";

test.describe("documentation navigation", () => {
  test("lists the issue tools only inside the docs menu", async ({ page }) => {
    await page.goto("/docs/vscode-cluster-panel/");

    await expect(
      page.locator(".site-header .nav-links").getByRole("link", {
        name: "Issue graph",
        exact: true,
      }),
    ).toHaveCount(0);

    const docsNav = page.locator(".docs-sidebar__nav");
    await expect(page.locator(".docs-sidebar").getByText("The Manuscript", { exact: true })).toHaveCount(0);
    await expect(page.locator(".docs-sidebar").getByText(/Live duplicate-code analysis/)).toHaveCount(0);
    await expect(docsNav.locator("details.docs-nav-group")).toHaveCount(4);
    await expect(docsNav.locator("a")).toHaveCount(11);
    await expect(
      docsNav.locator('[data-docs-group="guides"]'),
    ).toHaveAttribute("open", "");
    const trustGroup = docsNav.locator('[data-docs-group="trust"]');
    await expect(trustGroup.locator('a[href="/issues/"]')).toContainText("Issue graph");
    await expect(trustGroup.locator('a[href="/issues/planner/"]')).toContainText("Issue planner");
    await expect(trustGroup.locator('a[href="/issues/statistics/"]')).toHaveCount(0);
  });

  test("uses clear plus and minus accordion indicators", async ({ page }) => {
    await page.goto("/docs/vscode-cluster-panel/");
    const start = page.locator('[data-docs-group="start"]');
    const guides = page.locator('[data-docs-group="guides"]');
    const indicator = (locator) =>
      locator.locator("summary").evaluate((element) => {
        const style = getComputedStyle(element, "::after");
        return {
          content: style.content.replaceAll('"', ""),
          borderRightWidth: style.borderRightWidth,
          borderBottomWidth: style.borderBottomWidth,
          transform: style.transform,
        };
      });

    expect(await indicator(start)).toEqual({
      content: "+",
      borderRightWidth: "0px",
      borderBottomWidth: "0px",
      transform: "none",
    });
    expect((await indicator(guides)).content).toBe("−");
    await start.locator("summary").click();
    expect((await indicator(start)).content).toBe("−");
  });

  test("localizes the grouped documentation navigation", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/zh/docs/");

    const docsNav = page.locator(".docs-sidebar__nav");
    await expect(docsNav.locator("a")).toHaveCount(11);
    await expect(docsNav.locator("summary")).toHaveText([
      "入门",
      "使用指南",
      "参考",
      "原理与透明度",
    ]);
    await expect(docsNav.locator('a[href="/issues/"]')).toContainText("交互式图谱");
    await expect(docsNav.locator('a[href="/issues/planner/"]')).toContainText("问题规划器");
    await page.getByRole("button", { name: "Toggle menu" }).click();
    await expect(page.locator("body")).toHaveClass(/is-docs/);
    await expect(page.locator(".docs-sidebar")).toHaveClass(/open/);
    await expect(page.locator(".site-header .nav-links")).toBeHidden();
  });

  test("expands grouped docs navigation without overflowing a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/docs/vscode-cluster-panel/");
    await page.getByRole("button", { name: "Toggle menu" }).click();

    const sidebar = page.locator(".docs-sidebar");
    await expect(sidebar).toHaveClass(/open/);
    const trustGroup = sidebar.locator('[data-docs-group="trust"]');
    await expect(trustGroup).not.toHaveAttribute("open", "");
    await trustGroup.locator("summary").click();
    await expect(trustGroup).toHaveAttribute("open", "");

    const summaryHeight = await trustGroup
      .locator("summary")
      .evaluate((element) => element.getBoundingClientRect().height);
    expect(summaryHeight).toBeGreaterThanOrEqual(44);
    const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);

    await trustGroup.locator('a[href="/issues/"]').click();
    await expect(page).toHaveURL(/\/issues\/$/);
  });

  test("gives every issue document the same collapsible docs sidebar", async ({ page }) => {
    for (const path of ["/issues/", "/issues/planner/"]) {
      await page.goto(path);
      await expect(page.locator("body")).toHaveClass(/is-docs/);
      await expect(page.locator(".docs-sidebar__nav a")).toHaveCount(11);
      await expect(page.getByRole("button", { name: "Collapse documentation sidebar" })).toBeVisible();
    }

    const shell = page.locator(".docs-shell");
    const workspace = page.locator(".docs-workspace");
    const expandedWidth = (await workspace.boundingBox()).width;
    await page.getByRole("button", { name: "Collapse documentation sidebar" }).click();
    await expect(shell).toHaveClass(/is-sidebar-collapsed/);
    await expect(page.getByRole("button", { name: "Expand documentation sidebar" })).toHaveAttribute("aria-expanded", "false");
    expect((await workspace.boundingBox()).width).toBeGreaterThan(expandedWidth);
  });

  test("redirects the retired statistics route into the planner tab", async ({ page }) => {
    await page.goto("/issues/statistics/");
    await expect(page).toHaveURL(/\/issues\/planner\/\?view=statistics$/);
    await expect(page.getByRole("tab", { name: "Statistics" })).toHaveAttribute("aria-selected", "true");
  });
});

test.describe("issue atlas", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/issues/");
    await expect(page.locator("[data-visible-count]")).toContainText("Showing");
  });

  test("renders the relationship graph without dashboard content", async ({ page }) => {
    const report = await page.evaluate(async () => (await fetch("/assets/data/issues.json")).json());
    const populatedStreams = report.workstreams.filter((stream) => stream.count > 0);
    await expect(page.locator(".graph-node")).toHaveCount(report.issues.length);
    await expect(page.locator(".graph-halo")).toHaveCount(populatedStreams.length);
    await expect(page.locator(".graph-cluster-label")).toHaveText(populatedStreams.map((stream) => stream.name));
    await expect(page.locator(".graph-edge")).toHaveCount(report.relationships.length);
    expect(await page.locator(".graph-edge--blocks").count()).toBeGreaterThan(0);
    expect(await page.locator(".graph-edge--sub_issue").count()).toBeGreaterThan(0);
    await expect(page.locator(".graph-edge--blocks").first()).toHaveAttribute("marker-end", "url(#arrow-blocks)");
    await expect(page.locator(".graph-legend")).toContainText("Blocks →");
    await expect(page.locator(".graph-legend")).toContainText("Parent → sub-issue");
    await expect(page.locator(".atlas-hero")).toHaveCount(0);
    await expect(page.locator(".atlas-summary")).toHaveCount(0);
    await expect(page.locator(".verification-note")).toHaveCount(0);
    await expect(page.locator(".view-tabs")).toHaveCount(0);
    await expect(page.locator(".atlas-method")).toHaveCount(0);
    await expect(page.locator(".docs-sidebar")).toBeVisible();
  });

  test("fills the viewport below the header with no content above the graph", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.reload();
    const headerBox = await page.locator(".site-header").boundingBox();
    const sidebarBox = await page.locator(".docs-sidebar").boundingBox();
    const visualizerBox = await page.locator(".issue-atlas--graph").boundingBox();
    const stageBox = await page.locator(".atlas-stage").boundingBox();
    const canvasBox = await page.locator(".network-canvas").boundingBox();

    expect(visualizerBox.x).toBeCloseTo(sidebarBox.x + sidebarBox.width, 0);
    expect(visualizerBox.width).toBeCloseTo(1440 - sidebarBox.width, 0);
    expect(visualizerBox.y).toBeCloseTo(headerBox.y + headerBox.height, 0);
    expect(visualizerBox.height).toBeCloseTo(900 - headerBox.height, 0);
    expect(stageBox.y).toBeCloseTo(visualizerBox.y, 0);
    expect(canvasBox.width).toBeCloseTo(visualizerBox.width, 0);
    expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBeLessThanOrEqual(900);
  });

  test("cannot switch the graph-only page to a dashboard view", async ({ page }) => {
    await page.goto("/issues/?view=queue");
    await expect(page.locator(".network-svg")).toBeVisible();
    await expect(page.locator("[data-view-panel=queue]")).toHaveCount(0);
    await expect(page).not.toHaveURL(/view=queue/);
  });

  test("zooms the graph and opens useful issue detail", async ({ page }) => {
    const viewport = page.locator(".graph-viewport");
    await expect(viewport).toHaveAttribute("data-zoom", "1");
    await page.getByRole("button", { name: "Zoom in" }).click();
    await expect(viewport).not.toHaveAttribute("data-zoom", "1");
    const zoomedIn = Number(await viewport.getAttribute("data-zoom"));
    await page.getByRole("button", { name: "Zoom out" }).click();
    expect(Number(await viewport.getAttribute("data-zoom"))).toBeLessThan(zoomedIn);
    await page.getByRole("button", { name: "Reset graph position" }).click();
    await expect(viewport).toHaveAttribute("data-zoom", "1.00");
    const transform = await viewport.getAttribute("transform");
    const canvas = page.locator(".network-canvas");
    await canvas.scrollIntoViewIfNeeded();
    const box = await canvas.boundingBox();
    await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.5);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.45);
    await page.mouse.up();
    await expect(viewport).not.toHaveAttribute("transform", transform);
    const selectedNode = page.locator(".graph-node").first();
    await selectedNode.click();
    const drawer = page.locator("[data-issue-drawer]");
    await expect(drawer).toHaveAttribute("role", "dialog");
    await expect(drawer).toHaveAttribute("aria-hidden", "false");
    await expect(drawer).not.toHaveAttribute("inert", "");
    const issueLink = page.getByRole("link", { name: /Open the full issue on GitHub/ });
    const closeDrawer = page.getByRole("button", { name: "Close issue details" });
    await expect(issueLink).toBeVisible();
    await issueLink.focus();
    await issueLink.press("Tab");
    await expect(closeDrawer).toBeFocused();
    await closeDrawer.click();
    await expect(drawer).toHaveAttribute("aria-hidden", "true");
    await expect(drawer).toHaveAttribute("inert", "");
    await expect(selectedNode).toBeFocused();
  });

  test("keeps fixed-on-main text off nodes and in the hover bubble", async ({ page }) => {
    const fixedIssues = await page.evaluate(async () => {
      const response = await fetch("/assets/data/issues.json");
      const report = await response.json();
      return report.issues.flatMap((issue) => {
        const label = issue.labels.find((candidate) => candidate.name === "fixed-on-main");
        return label ? [{ number: issue.number, color: `#${label.color}`, description: label.description }] : [];
      });
    });

    expect(fixedIssues.length).toBeGreaterThan(0);
    const fixedNodes = page.locator(".graph-node--fixed-on-main");
    await expect(fixedNodes).toHaveCount(fixedIssues.length);
    await expect(page.locator(".graph-node__status")).toHaveCount(0);
    for (const issue of fixedIssues) {
      const node = page.locator(`.graph-node[data-issue="${issue.number}"]`);
      await expect(node).toHaveClass(/graph-node--fixed-on-main/);
      expect(
        await node.evaluate((element) =>
          element.style.getPropertyValue("--fixed-on-main-color"),
        ),
      ).toBe(issue.color);
    }
    expect(
      await fixedNodes
        .first()
        .locator(".graph-node__dot")
        .evaluate((element) => getComputedStyle(element).filter),
    ).not.toBe("none");
    await fixedNodes.first().hover();
    await expect(page.locator(".graph-tooltip")).toHaveClass(/is-visible/);
    const fixedChip = page.locator(".graph-tooltip__labels .label-chip").filter({ hasText: "fixed-on-main" });
    await expect(fixedChip).toBeVisible();
    await expect(fixedChip).toHaveAttribute("title", fixedIssues[0].description);
  });

  test("keeps assignee avatars off nodes and in the hover bubble", async ({ page }) => {
    const assignedIssues = await page.evaluate(async () => {
      const response = await fetch("/assets/data/issues.json");
      const report = await response.json();
      return report.issues.flatMap((issue) => {
        const assignee = issue.assignees.find((candidate) => candidate.avatar);
        return assignee
          ? [{ number: issue.number, login: assignee.login, avatar: assignee.avatar }]
          : [];
      });
    });

    expect(assignedIssues.length).toBeGreaterThan(0);
    await expect(page.locator(".graph-node__assignee, .graph-node image")).toHaveCount(0);
    const assigned = assignedIssues[0];
    const node = page.locator(`.graph-node[data-issue="${assigned.number}"]`);
    await expect(node).toHaveAttribute("aria-label", new RegExp(assigned.login));
    await node.hover();
    await expect(page.locator(".graph-tooltip__assignee")).toContainText(`@${assigned.login}`);
    await expect(page.locator(".graph-tooltip__assignee img")).toHaveAttribute("src", assigned.avatar);
  });

  test("filters fixed-on-main into the release verification queue", async ({ page }) => {
    await page.locator(".graph-filter-panel summary").click();
    await page.selectOption('select[name="label"]', "fixed-on-main");
    await expect(page.locator("[data-visible-count]")).toContainText(/Showing \d+ of \d+ issues/);
    const nodes = page.locator(".graph-node");
    expect(await nodes.count()).toBeGreaterThan(0);
    await expect(nodes.first()).toHaveClass(/graph-node--verify/);
    await nodes.first().click();
    await expect(page.locator(".drawer-kicker")).toContainText("verify next release");
    await expect(page.locator(".drawer-priority")).toContainText("Verify next release");
  });

  test("searches by issue number without losing context", async ({ page }) => {
    const issueNumber = await page.locator(".graph-node").first().getAttribute("data-issue");
    await page.locator(".graph-filter-panel summary").click();
    await page.locator('input[name="search"]').fill(`#${issueNumber}`);
    await expect(page.locator(".graph-node")).toHaveCount(1);
    await expect(page.locator("[data-visible-count]")).toContainText("Showing 1 of");
  });

  test("keeps the primary experience usable on a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.reload();
    await expect(page.locator(".network-svg")).toBeVisible();
    await expect(page.locator(".atlas-hero")).toHaveCount(0);
    const canvas = page.locator(".network-canvas");
    const canvasBox = await canvas.boundingBox();
    expect(canvasBox.y).toBeLessThan(844);
    expect(canvasBox.height).toBeGreaterThanOrEqual(300);
    for (const button of await page.locator(".network-tool").all()) {
      const box = await button.boundingBox();
      expect(box.width).toBeGreaterThanOrEqual(44);
      expect(box.height).toBeGreaterThanOrEqual(44);
    }
    await page.locator(".graph-node").first().click();
    const closeBox = await page
      .getByRole("button", { name: "Close issue details" })
      .boundingBox();
    expect(closeBox.width).toBeGreaterThanOrEqual(44);
    expect(closeBox.height).toBeGreaterThanOrEqual(44);
    const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);
  });
});

test.describe("issue planner document", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/issues/planner/");
    await expect(page.locator("[data-visible-count]")).toContainText("Showing");
  });

  test("fills the remaining viewport with exactly one active planner tab", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.reload();
    const headerBox = await page.locator(".site-header").boundingBox();
    const sidebarBox = await page.locator(".docs-sidebar").boundingBox();
    const workspaceBox = await page.locator(".docs-workspace").boundingBox();
    const appBox = await page.locator(".atlas-app--planner").boundingBox();
    const tabsBox = await page.locator(".view-tabs").boundingBox();
    const stageBox = await page.locator(".atlas-stage").boundingBox();
    const panelBox = await page.locator("[data-view-panel]").boundingBox();

    expect(workspaceBox.x).toBeCloseTo(sidebarBox.x + sidebarBox.width, 0);
    expect(workspaceBox.width).toBeCloseTo(1440 - sidebarBox.width, 0);
    expect(workspaceBox.height).toBeCloseTo(900 - headerBox.height, 0);
    expect(appBox.width).toBeCloseTo(workspaceBox.width, 0);
    expect(appBox.height).toBeCloseTo(workspaceBox.height, 0);
    expect(panelBox.width).toBeCloseTo(stageBox.width, 0);
    expect(panelBox.height).toBeCloseTo(stageBox.height, 0);
    expect(tabsBox.width).toBeLessThan(workspaceBox.width * 0.6);
    await expect(page.locator(".view-tab")).toHaveText(["Priority", "Queue", "Statistics", "Runway"]);
    expect(await page.locator(".view-tab").evaluateAll((tabs) => tabs.map((tab) => tab.getAttribute("aria-controls")))).toEqual([
      "panel-board", "panel-queue", "panel-statistics", "panel-runway",
    ]);
    await expect(page.locator("[data-view-panel]")).toHaveCount(1);
    await expect(page.getByRole("tab", { name: "Priority", exact: true })).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("[data-view-panel=board]")).toHaveAttribute("aria-labelledby", "tab-board");
    await expect(page).not.toHaveURL(/view=/);
    expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBeLessThanOrEqual(900);
  });

  test("supports keyboard navigation across tabs in visual order", async ({ page }) => {
    const priority = page.getByRole("tab", { name: "Priority", exact: true });
    await priority.focus();
    await priority.press("ArrowRight");
    await expect(page.getByRole("tab", { name: "Queue", exact: true })).toBeFocused();
    await expect(page.locator("[data-view-panel=queue]")).toBeVisible();
    await page.getByRole("tab", { name: "Queue", exact: true }).press("End");
    await expect(page.getByRole("tab", { name: "Runway" })).toBeFocused();
    await expect(page.locator("[data-view-panel=runway]")).toBeVisible();
    await page.getByRole("tab", { name: "Runway" }).press("Home");
    await expect(priority).toBeFocused();
    await expect(page.locator("[data-view-panel=board]")).toBeVisible();
  });

  test("switches between priority, queue, statistics, and runway", async ({ page }) => {
    await expect(page.locator("[data-view-panel=board] .board-lane").first()).toBeVisible();
    await page.getByRole("tab", { name: "Queue", exact: true }).click();
    await expect(page.locator("[data-view-panel=queue] tbody tr").first()).toBeVisible();
    await expect(page.locator("[data-view-panel]")).toHaveCount(1);
    await expect(page).toHaveURL(/view=queue/);
    await page.getByRole("tab", { name: "Statistics" }).click();
    await expect(page.locator("[data-view-panel=statistics]")).toBeVisible();
    await page.getByRole("tab", { name: "Runway" }).click();
    await expect(page.locator("[data-view-panel=runway] .runway-bar").first()).toBeVisible();
    await expect(page.locator("[data-view-panel]")).toHaveCount(1);
  });

  test("renders GitHub label colors with readable text", async ({ page }) => {
    const chips = page.locator(".priority-board .label-chip");
    expect(await chips.count()).toBeGreaterThan(0);
    const contrasts = await chips.evaluateAll((elements) => elements.map((element) => {
      const luminance = (color) => {
        const values = color.match(/[\d.]+/g).slice(0, 3).map((value) => Number(value) / 255);
        const linear = values.map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
        return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
      };
      const style = getComputedStyle(element);
      const foreground = luminance(style.color);
      const background = luminance(style.backgroundColor);
      return (Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05);
    }));
    expect(Math.min(...contrasts)).toBeGreaterThanOrEqual(4.5);
  });

  test("keeps every priority card inside its own lane", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.reload();
    const violations = await page.locator(".board-lane").evaluateAll((lanes) =>
      lanes.flatMap((lane) => {
        const laneBounds = lane.getBoundingClientRect();
        return [...lane.querySelectorAll(".issue-card")].flatMap((card) => {
          const cardBounds = card.getBoundingClientRect();
          const escapes = cardBounds.left < laneBounds.left || cardBounds.right > laneBounds.right;
          return escapes ? [`${card.dataset.issue}: ${cardBounds.left}-${cardBounds.right} outside ${laneBounds.left}-${laneBounds.right}`] : [];
        });
      }),
    );
    expect(violations, violations.join("\n")).toEqual([]);
  });

  test("shows compact filter-aware statistics inside the planner", async ({ page }) => {
    const report = await page.evaluate(async () => (await fetch("/assets/data/issues.json")).json());
    await page.getByRole("tab", { name: "Statistics" }).click();
    await expect(page.getByRole("heading", { name: "Issue statistics" })).toBeVisible();
    await expect(page.locator(".summary-card")).toHaveCount(5);
    await expect(page.locator("[data-summary=open]")).toHaveText(String(report.summary.open));
    await expect(page.getByText("“fixed-on-main” is a verification state, not done.")).toBeVisible();
    await expect(page.locator(".statistics-source")).toContainText("No AI enrichment");
    for (const card of await page.locator(".summary-card").all()) {
      expect((await card.boundingBox()).height).toBeLessThan(200);
    }
    await page.selectOption('select[name="label"]', "fixed-on-main");
    await expect(page.locator("[data-summary=open]")).toHaveText(String(report.summary.verify));
    await expect(page.locator("[data-summary=verify]")).toHaveText(String(report.summary.verify));
  });

  test("labels the runway as indicative and shows no schedule dates", async ({ page }) => {
    await page.getByRole("tab", { name: "Runway" }).click();
    await expect(page.locator(".runway-notice")).toContainText(
      "Indicative only — not a schedule",
    );
    await expect(page.locator(".runway-step")).not.toHaveCount(0);
    await expect(page.locator(".runway-week")).toHaveCount(0);
    await expect(page.locator("[data-generated-date]")).toHaveCount(0);

    const runwayText = await page.locator("[data-view-panel=runway]").innerText();
    expect(runwayText).not.toMatch(
      /\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Sept|Oct|Nov|Dec)\b/,
    );
    expect(runwayText).not.toMatch(/\b20\d{2}-\d{2}-\d{2}\b/);

    await page.locator("[data-view-panel=runway] .runway-bar").first().click();
    await expect(page.locator("[data-issue-drawer]")).toHaveClass(/is-open/);
    await expect(
      page.locator("[data-issue-drawer] .drawer-fact").filter({ hasText: "Updated" }),
    ).toHaveCount(0);
    await page.getByRole("button", { name: "Close issue details" }).click();

    await page.getByRole("tab", { name: "Queue", exact: true }).click();
    await expect(page.getByRole("columnheader", { name: "Updated" })).toHaveCount(0);
  });

  test("keeps every planner tab usable on a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.reload();
    for (const view of ["Priority", "Queue", "Statistics", "Runway"]) {
      await page.getByRole("tab", { name: view }).click();
      await expect(page.locator("[data-view-panel]")).toHaveCount(1);
      const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
      const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
      expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);
    }
  });
});
