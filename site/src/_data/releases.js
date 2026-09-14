// `pin` below is the version every documented `uses:` pin and `version:` input
// renders. It is held nowhere in git: the site is rebuilt and deployed after the
// GitHub release exists, so resolving it here is what makes a committed pin
// structurally unable to go stale. When the API is unreachable — an offline
// build, a rate limit — it degrades to the same substitute-your-own token the
// README shows, never to a broken `@v` with no version after it.
//
// `default` must stay this file's ONLY export. Eleventy hands templates the
// module namespace of a `_data` file that exports anything else, never calling
// the function, which blanks every `{{ releases.* }}` on the site silently and
// at exit 0. Pinned by `test-action-contract.mjs`. [ACTION-VERSION-DOCS]
import { PIN_PLACEHOLDER } from "../../../scripts/release/stamp-release-version.mjs";

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
    pin: latest ? latest.version : PIN_PLACEHOLDER,
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
