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
  pollFunc: () => void,
  pollRate = 100,
): Promise<F> {
  // an arbitrary marker object
  const sleepRv = {};

  while (true) {
    // a promise that resolves after [pollRate] ms, and returns the [sleepRv] object
    const sleepPromise: Promise<typeof sleepRv> = new Promise((resolve) =>
      setTimeout(() => resolve(sleepRv), pollRate),
    );

    // either wait for the promise to return, or timeout after [pollRate] ms
    const rv = await Promise.race([promise, sleepPromise]);
    if (rv !== sleepRv) {
      // function finished
      return rv as F;
    }

    // timeout
    await pollFunc();
  }
}

/**
 * Given an element, return the size of `1em` in pixels.
 * @param element The element to derive CSS styles from
 */
export function getEmSizeInPx(element: Element) {
  return parseFloat(getComputedStyle(element).fontSize);
}

/** Number of pixels in `1rem` */
const REM_PIXELS = getEmSizeInPx(document.documentElement);

/**
 * Convert a size definition in `rem` units like `0.875rem` into pixels.
 * @param emSize
 */
export function parseRemSize(emSize: string): number | null {
  const match = emSize.match(/^ *(\d+(?:\.\d+)?) *rem *$/);
  if (match === null) {
    return null;
  } else {
    const rem = parseFloat(match[1]);
    return rem * REM_PIXELS;
  }
}

export type EventListenerInfo = [
  element: Element | Window,
  type: string,
  listener: (this: Element, ev: Event) => void,
];

export function createEventListenerRegistry() {
  const eventListeners: EventListenerInfo[] = [];

  function add<T extends Event>(
    element: Element | Window,
    type: string,
    _listener: (this: Element, ev: T) => void,
  ): EventListenerInfo {
    // manually force the type of 2nd parameter to be "Event"
    // because the typing annotation of `addEventListener` is fucking wrong
    const listener = _listener as (this: Element, ev: Event) => void;

    element.addEventListener(type, listener);
    const info: EventListenerInfo = [element, type, listener];
    eventListeners.push(info);
    console.log("Added event listener:", element, type, listener);
    return info;
  }

  function remove(listenerInfo: EventListenerInfo) {
    const infoIdx = eventListeners.indexOf(listenerInfo);
    if (infoIdx === -1) {
      const [element, type, listener] = listenerInfo;
      element.removeEventListener(type, listener);
      eventListeners.splice(infoIdx, 1);
      console.log("Removed event listener:", element, type, listener);
    } else {
      console.error(
        "No registered event listener with given specification:",
        listenerInfo,
      );
      throw "No registered event listener with given specification";
    }
  }

  function clear() {
    for (const [element, type, listener] of eventListeners) {
      element.removeEventListener(type, listener);
      console.log("Removed event listener:", element, type, listener);
    }
    // setting length to 0 clears the array
    eventListeners.length = 0;
  }

  return {
    add: add,
    remove: remove,
    clear: clear,
  };
}

/**
 * Returns the index of the element in the given array that is closest to the
 * specified number.
 *
 * @param arr - The array of numbers in increasing order.
 * @param num - The number to find the closest element to.
 * @returns The index of the closest element in the array.
 */
export function findClosestIndex(arr: number[], num: number) {
  let minDiff = Infinity;
  let closestIndex = -1;

  for (let i = 0; i < arr.length; i++) {
    const diff = Math.abs(num - arr[i]);
    if (diff < minDiff) {
      minDiff = diff;
      closestIndex = i;
    }
  }

  return closestIndex;
}

export function unreachable(x: never): never {
  throw new Error("This statement should never be reached.");
}

export function tagsToString(tags: string[]): string {
  const result = [];
  for (const tag of tags) {
    const hasSpace = tag.indexOf(" ") !== -1;
    if (hasSpace) {
      result.push(`"${tag}"`);
    } else {
      result.push(tag);
    }
  }
  return result.join(" ");
}
