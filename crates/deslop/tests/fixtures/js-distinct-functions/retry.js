// Retry an async operation with exponential backoff and jitter.
export async function withRetry(operation, attempts = 5) {
  let lastError;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      const backoff = 2 ** attempt * 100;
      const jitter = Math.floor(Math.random() * 50);
      await new Promise((resolve) => setTimeout(resolve, backoff + jitter));
    }
  }
  throw lastError;
}
