export async function loadUserProfile(userId, client) {
  const session = await client.authenticate();
  const profile = await client.get(`/users/${userId}`, session);
  const settings = await client.get(`/users/${userId}/settings`, session);
  const merged = Object.assign({}, profile, settings);
  await client.close(session);
  return merged;
}
