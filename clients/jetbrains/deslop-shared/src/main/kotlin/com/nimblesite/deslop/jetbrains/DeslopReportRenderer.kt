package com.nimblesite.deslop.jetbrains

/**
 * Renders the live Deslop HTML report by asking the running `deslop-lsp` server
 * (through whichever LSP client the IDE family uses) to execute its report-render
 * command. Implemented as a project service so the shared report UI — the
 * [DeslopReportToolWindowFactory] tool window and [DeslopRenderReportAction] — can
 * render without depending on the LSP4IJ client APIs directly. This is the single
 * seam a future native-LSP surface would re-implement.
 */
interface DeslopReportRenderer {
    /**
     * Runs the render command and returns the report HTML, or null when no server
     * is running or it produced no report. May block on the LSP response, so call
     * it off the EDT.
     */
    fun render(): String?

    /**
     * Returns the live structured report as a JSON string for native rendering
     * (the worst-offenders grouping tree the tool window shows when the IDE has no
     * embedded browser), or null when no server is running or it produced no
     * report. Like [render] it may block on the LSP, so call it off the EDT.
     */
    fun reportJson(): String?
}
