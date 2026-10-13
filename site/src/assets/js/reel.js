// [SITE-HOME-VIDEO] One page clock drives every scene and cue, so the reel loops, pauses and holds on a single timeline.
const MILLISECONDS_PER_SECOND = 1000;
const START = 0;
const HIDDEN = 0;
const VISIBLE = 1;
const FADE_SECONDS = 0.35;
const ENTER_SECONDS = 0.45;
const OFFSET_PIXELS = 16;
const DEFAULT_EFFECT = "rise";
const REDUCED_MOTION = "(prefers-reduced-motion: reduce)";
const STILL = [{ opacity: HIDDEN }, { opacity: VISIBLE }];
const EFFECTS = {
  rise: [{ opacity: HIDDEN, transform: `translateY(${OFFSET_PIXELS}px)` }, { opacity: VISIBLE, transform: "none" }],
  grow: [{ opacity: HIDDEN, transform: "scaleX(0)" }, { opacity: VISIBLE, transform: "none" }],
  fade: STILL,
};

// Explicit first and last keyframes stop the browser easing in from the element's resting style.
function timeline(element, duration, frames) {
  const padded = [{ ...frames[0], at: START }, ...frames, { ...frames.at(-1), at: duration }];
  const keyframes = padded.map(({ at, ...style }) => ({ ...style, offset: at / duration }));
  const animation = element.animate(keyframes, { duration: duration * MILLISECONDS_PER_SECOND, fill: "both" });
  animation.pause();
  return animation;
}

function sceneAnimation(scene, duration, [hidden, shown]) {
  const start = Number(scene.dataset.start);
  const end = Number(scene.dataset.end);
  return timeline(scene, duration, [
    { ...hidden, at: start },
    { ...shown, at: start + FADE_SECONDS },
    { ...shown, at: end - FADE_SECONDS },
    { opacity: HIDDEN, at: end },
  ]);
}

function cueAnimation(element, start, duration, reducedMotion) {
  const [hidden, shown] = reducedMotion ? STILL : EFFECTS[element.dataset.effect ?? DEFAULT_EFFECT];
  const at = start + Number(element.dataset.cue);
  return timeline(element, duration, [{ ...hidden, at }, { ...shown, at: at + ENTER_SECONDS }]);
}

function sceneAnimations(scene, duration, reducedMotion) {
  const start = Number(scene.dataset.start);
  const cues = [...scene.querySelectorAll("[data-cue]")];
  return [
    sceneAnimation(scene, duration, reducedMotion ? STILL : EFFECTS[DEFAULT_EFFECT]),
    ...cues.map((element) => cueAnimation(element, start, duration, reducedMotion)),
  ];
}

function createClock(duration, opening) {
  let origin = null;
  let held = opening;
  const seconds = () => (origin === null ? held : ((performance.now() - origin) / MILLISECONDS_PER_SECOND) % duration);
  return {
    seconds,
    running: () => origin !== null,
    start: () => { origin ??= performance.now() - held * MILLISECONDS_PER_SECOND; },
    stop: () => { held = seconds(); origin = null; },
  };
}

function createReel(player) {
  const duration = Number(player.dataset.duration);
  const reducedMotion = matchMedia(REDUCED_MOTION).matches;
  const scenes = [...player.querySelectorAll("[data-scene]")];
  return {
    player,
    scenes,
    toggle: player.querySelector('[data-action="play"]'),
    clock: createClock(duration, Number(player.dataset.opening)),
    animations: scenes.flatMap((scene) => sceneAnimations(scene, duration, reducedMotion)),
    held: reducedMotion,
    onScreen: false,
  };
}

function setAttribute(element, name, value) {
  if (element.getAttribute(name) !== value) element.setAttribute(name, value);
}

function render(reel) {
  const seconds = reel.clock.seconds();
  const running = reel.clock.running();
  const active = reel.scenes.findLast((scene) => Number(scene.dataset.start) <= seconds);
  reel.player.dataset.position = String(seconds);
  setAttribute(reel.player, "data-playing", String(running));
  setAttribute(reel.toggle, "aria-label", running ? reel.toggle.dataset.pauseLabel : reel.toggle.dataset.playLabel);
  reel.scenes.forEach((scene) => setAttribute(scene, "aria-hidden", String(scene !== active)));
  reel.animations.forEach((animation) => { animation.currentTime = seconds * MILLISECONDS_PER_SECOND; });
}

function loop(reel) {
  render(reel);
  if (reel.clock.running()) reel.frame = requestAnimationFrame(() => loop(reel));
}

function resume(reel) {
  if (reel.clock.running()) return;
  reel.clock.start();
  loop(reel);
}

function halt(reel) {
  cancelAnimationFrame(reel.frame);
  reel.clock.stop();
  render(reel);
}

// The reel only runs while it is on screen, the tab is visible and the visitor has not paused it.
function sync(reel) {
  if (reel.held || document.hidden || !reel.onScreen) halt(reel);
  else resume(reel);
}

function togglePlayback(reel) {
  reel.held = reel.clock.running();
  sync(reel);
}

function initialise(player) {
  const reel = createReel(player);
  reel.toggle.addEventListener("click", () => togglePlayback(reel));
  document.addEventListener("visibilitychange", () => sync(reel));
  new IntersectionObserver(([entry]) => {
    reel.onScreen = entry.isIntersecting;
    sync(reel);
  }).observe(player);
  player.querySelector(".motion-reel__controls").hidden = false;
  render(reel);
}

document.querySelectorAll(".motion-reel").forEach(initialise);
