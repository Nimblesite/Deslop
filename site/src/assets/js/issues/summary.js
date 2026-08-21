const root = document.querySelector("[data-issue-statistics]");

async function loadSummary() {
  try {
    const response = await fetch("/assets/data/issues.json");
    if (!response.ok) throw new Error(`Issue data request failed (${response.status}).`);
    const report = await response.json();
    for (const [name, value] of Object.entries(report.summary)) {
      const target = root.querySelector(`[data-summary="${name}"]`);
      if (target) target.textContent = String(value);
    }
  } catch (error) {
    root.querySelectorAll("[data-summary]").forEach((target) => { target.textContent = "?"; });
    console.error(error);
  }
}

loadSummary();
