/**
 * Mobile navigation drawer — cooperative enhancement of techdoc's mobile menu.
 *
 * techdoc's main.js binds the hamburger to toggle `body.menu-open` (and, on docs
 * pages, `.docs-sidebar.open`). Our CSS drives the off-canvas drawer + scrim from
 * `body.menu-open` alone. This module adds the dismissal affordances that the
 * package leaves out: tapping the scrim, pressing Escape, following a link, or
 * crossing into the desktop breakpoint all close the drawer. Closing resets the
 * toggle's `aria-expanded` so techdoc's next click re-opens cleanly.
 */
const desktop = window.matchMedia("(min-width: 768px)");

const toggle = document.getElementById('mobile-menu-toggle');
const scrim = document.querySelector('.drawer-scrim');
const drawers = document.querySelectorAll('.docs-sidebar, .nav-links');
const docsShell = document.querySelector('.docs-shell');
const docsCollapse = document.querySelector('[data-docs-sidebar-collapse]');
const siteLinks = document.querySelector('.nav-links');
const headerActions = document.querySelector('.nav-actions');
const docsNav = document.querySelector('.docs-sidebar__nav');

function placeSiteLinks() {
  closeDrawer();
  if (!docsNav) return;
  const sibling = desktop.matches ? headerActions : docsNav;
  sibling.before(siteLinks);
}

placeSiteLinks();
desktop.addEventListener('change', placeSiteLinks);

docsCollapse?.addEventListener('click', () => {
  const collapsed = docsShell.classList.toggle('is-sidebar-collapsed');
  docsCollapse.setAttribute('aria-expanded', String(!collapsed));
  docsCollapse.setAttribute('aria-label', collapsed ? 'Expand documentation sidebar' : 'Collapse documentation sidebar');
  docsCollapse.querySelector('span').textContent = collapsed ? '›' : '‹';
});

function closeDrawer() {
  if (!document.body.classList.contains('menu-open')) return;
  document.body.classList.remove('menu-open');
  drawers.forEach((drawer) => drawer.classList.remove('open'));
  toggle?.setAttribute('aria-expanded', 'false');
}

scrim?.addEventListener('click', closeDrawer);

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') closeDrawer();
});

drawers.forEach((drawer) => {
  drawer.addEventListener('click', (event) => {
    if (event.target.closest('a')) closeDrawer();
  });
});
