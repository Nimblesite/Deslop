const RELEASES_URL = "https://github.com/Nimblesite/Deslop/releases";
const LATEST_RELEASE_URL = `${RELEASES_URL}/latest`;
const API_URL = "https://api.github.com/repos/Nimblesite/Deslop/releases?per_page=12";

const buildHeaders = () => {
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "deslop-site-build",
  };
  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
  if (token) headers.Authorization = `Bearer ${token}`;
  return headers;
};

const assetData = (asset) => ({
  name: asset.name,
  url: asset.browser_download_url,
  size: asset.size,
  downloadCount: asset.download_count,
});

const releaseData = (release) => ({
  id: release.id,
  tag: release.tag_name,
  version: release.tag_name.startsWith("v") ? release.tag_name.slice(1) : release.tag_name,
  name: release.name || release.tag_name,
  url: release.html_url,
  prerelease: release.prerelease,
  publishedAt: release.published_at,
  publishedDate: new Date(release.published_at),
  assets: (release.assets || []).map(assetData),
});

const payload = (items, error = null) => {
  const latest = items.find((release) => !release.prerelease) || items[0] || null;
  const generatedAt = new Date();
  return {
    allUrl: RELEASES_URL,
    latestUrl: LATEST_RELEASE_URL,
    generatedAt,
    generatedAtIso: generatedAt.toISOString(),
    latest,
    items: items.map((release) => ({
      ...release,
      isLatestStable: Boolean(latest && release.id === latest.id),
    })),
    error,
  };
};

export default async function releases() {
  try {
    const response = await fetch(API_URL, { headers: buildHeaders() });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const body = await response.json();
    return payload(Array.isArray(body) ? body.map(releaseData) : []);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.warn(`GitHub releases metadata unavailable: ${message}`);
    return payload([], message);
  }
}
