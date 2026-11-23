const period = 30_000;

export async function freshTotpTimestamp(poll, previous, clock = Date.now) {
  let timestamp;
  await poll(() => {
    timestamp = clock();
    const counter = Math.floor(timestamp / period);
    const elapsed = timestamp % period;
    return counter > previous && elapsed >= 250 && elapsed <= period - 3_000;
  }, 35_000);
  return timestamp;
}
