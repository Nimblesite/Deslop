import fs from 'node:fs';
// [DOC-FILMS-DELIVERY] The delivered film catalog and the page each film plays on.
export const FILMS=JSON.parse(fs.readFileSync(new URL('../../src/_data/docFilms.json',import.meta.url),'utf8'));

export function filmRoute(slug) {
  if (slug === 'osprey') return '/blog/osprey-first-class/';
  if (slug === 'getting-started') return '/docs/';
  return `/docs/${slug}/`;
}
