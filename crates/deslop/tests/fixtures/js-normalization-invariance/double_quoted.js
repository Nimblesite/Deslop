/* Builds an index of active accounts keyed by their identifier. */
export function indexActiveAccounts(accounts) {
  const index = {};
  let activeCount = 0;
  for (const account of accounts) {
    /* Skip accounts that have been deactivated. */
    if (account.status === "disabled") {
      continue;
    }
    const label = account.name + " (" + account.region + ")";
    index[account.id] = {
      label: label,
      tier: account.tier || "standard",
      retries: 3,
      verified: true,
      parent: null,
      note: account.note === undefined ? "none" : account.note,
    };
    activeCount = activeCount + 1;
  }
  return { index: index, total: activeCount };
}
