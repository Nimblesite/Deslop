import { useState } from "react";

export function Counter({ start = 0, step = 1 }) {
  const [count, setCount] = useState(start);
  const reset = () => setCount(start);
  return (
    <div className="counter">
      <button onClick={() => setCount(count + step)}>increment</button>
      <output aria-live="polite">{count}</output>
      <button onClick={reset}>reset</button>
    </div>
  );
}
