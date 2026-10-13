// [SITE-HOME-VIDEO] Audio clocks the DOM scenes; no encoded video is loaded.
const MILLISECONDS_PER_SECOND = 1000;
const START = 0;
const VISIBLE = 1;
const FADE_SECONDS = 0.35;
const OFFSET_PIXELS = 16;
const REDUCED_MOTION = "(prefers-reduced-motion: reduce)";
const CANCELLED_PLAYBACK = "AbortError";

function sceneAnimation(scene, duration, reducedMotion) {
  const start = Number(scene.dataset.start);
  const end = Number(scene.dataset.end);
  const animation = scene.animate([
    { opacity: start === START ? VISIBLE : START, transform: `translateY(${reducedMotion || start === START ? START : OFFSET_PIXELS}px)`, offset: start / duration },
    { opacity: VISIBLE, transform: "translateY(0)", offset: (start + FADE_SECONDS) / duration },
    { opacity: VISIBLE, offset: (end - FADE_SECONDS) / duration },
    { opacity: end === duration ? VISIBLE : START, offset: end / duration },
  ], { duration: duration * MILLISECONDS_PER_SECOND, fill: "both" });
  animation.pause();
  return animation;
}

function render(player, audio, scenes, animations) {
  const seconds = audio.currentTime;
  const active = scenes.findLast((scene) => Number(scene.dataset.start) <= seconds);
  player.dataset.position = String(seconds);
  player.querySelector("input").value = String(seconds);
  scenes.forEach((scene, index) => {
    scene.setAttribute("aria-hidden", String(scene !== active));
    animations[index].currentTime = seconds * MILLISECONDS_PER_SECOND;
  });
  const play = player.querySelector('[data-action="play"]');
  play.textContent = audio.paused ? play.dataset.playLabel : play.dataset.pauseLabel;
}

function playbackLoop(player, audio, scenes, animations) {
  let frame;
  const update = () => {
    render(player, audio, scenes, animations);
    if (!audio.paused) frame = requestAnimationFrame(update);
  };
  audio.addEventListener("play", update);
  audio.addEventListener("pause", () => { cancelAnimationFrame(frame); update(); });
  audio.addEventListener("seeked", () => render(player, audio, scenes, animations));
  audio.addEventListener("ended", () => render(player, audio, scenes, animations));
  document.addEventListener("visibilitychange", () => { if (document.hidden) audio.pause(); });
  render(player, audio, scenes, animations);
}

async function togglePlayback(audio, output) {
  if (!audio.paused) return audio.pause();
  output.textContent = "";
  try { await audio.play(); }
  catch (error) {
    if (error instanceof DOMException && error.name === CANCELLED_PLAYBACK && audio.paused) return;
    output.textContent = output.dataset.errorLabel;
  }
}

function controls(player, audio, scenes, animations) {
  const output = player.querySelector("output");
  player.querySelector('[data-action="play"]').addEventListener("click", () => togglePlayback(audio, output));
  player.querySelector('[data-action="restart"]').addEventListener("click", () => { audio.pause(); audio.currentTime = START; render(player, audio, scenes, animations); });
  player.querySelector('[data-action="mute"]').addEventListener("click", (event) => { audio.muted = !audio.muted; event.currentTarget.setAttribute("aria-pressed", String(audio.muted)); });
  player.querySelector("input").addEventListener("input", (event) => { audio.currentTime = Number(event.currentTarget.value); render(player, audio, scenes, animations); });
  player.querySelectorAll("button, input").forEach((control) => { control.disabled = false; });
}

function initialise(player) {
  const audio = player.querySelector("audio");
  const scenes = [...player.querySelectorAll("[data-scene]")];
  const reducedMotion = matchMedia(REDUCED_MOTION).matches;
  const animations = scenes.map((scene) => sceneAnimation(scene, Number(player.dataset.duration), reducedMotion));
  playbackLoop(player, audio, scenes, animations);
  controls(player, audio, scenes, animations);
}

document.querySelectorAll(".motion-reel").forEach(initialise);
