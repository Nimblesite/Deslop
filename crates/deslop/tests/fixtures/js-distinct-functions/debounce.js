// Produce a debounced wrapper that fires once after the calls go quiet.
export function debounce(fn, waitMs) {
  let timer = null;
  return function debounced(...args) {
    if (timer !== null) {
      clearTimeout(timer);
    }
    timer = setTimeout(() => {
      timer = null;
      fn.apply(this, args);
    }, waitMs);
  };
}
