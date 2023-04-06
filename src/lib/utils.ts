/**
 * An async delay function. Example usage:
 * ```
 * await sleep(1000);
 * ```
 * @param ms How many milliseconds to sleep for.
 */
export function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Poll a promise repeatedly until it completes.
 * @param promise The promise, this is the one we are waiting for.
 * @param pollFunc A short async function for polling, this runs multiple times while waiting for the long function.
 * @param pollRate How fast to poll the function, measured in ms.
 */
export async function pollUntilComplete<F>(
  promise: Promise<F>,
  pollFunc: () => any,
  pollRate = 100
): Promise<F> {
  const sleepRv = {};
  while (true) {
    // a promise that resolves after [pollRate] ms, and returns the [sleepRv] object
    const sleepPromise = new Promise((resolve) =>
      setTimeout(() => resolve(sleepRv), pollRate)
    );

    const rv: any = await Promise.race([promise, sleepPromise]);
    if (rv !== sleepRv) {
      // function finished
      return rv;
    }

    // timeout
    await pollFunc();
  }
}
