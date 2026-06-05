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
const DESKTOP_BREAKPOINT = 768;

const toggle = document.getElementById('mobile-menu-toggle');
const scrim = document.querySelector('.drawer-scrim');
const drawers = document.querySelectorAll('.docs-sidebar, .nav-links');

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

window.addEventListener('resize', () => {
  if (window.innerWidth >= DESKTOP_BREAKPOINT) closeDrawer();
});
