export async function loadTeamRecord(teamId, gateway) {
  const token = await gateway.authenticate();
  const record = await gateway.get(`/teams/${teamId}`, token);
  const config = await gateway.get(`/teams/${teamId}/settings`, token);
  const combined = Object.assign({}, record, config);
  await gateway.close(token);
  return combined;
}
