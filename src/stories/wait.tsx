// Stories whose content arrives late (a mocked command, an editor popup)
// mark themselves pending until it shows, so the screenshot run waits
import { useEffect, useRef, useState } from "react";

/**
 * Runs `run` once, then stays pending until `selector` matches on the page
 * (editor tooltips sit on the body, outside the story). The gallery waits
 * while any `[data-story-pending]` is on the page.
 */
export function WaitFor({ selector, run }: { selector?: string; run?: () => Promise<void> }) {
  const [done, setDone] = useState(false);
  // Once per mount: the story re-renders as `run` changes the store
  const first = useRef({ selector, run });
  useEffect(() => {
    const { selector, run } = first.current;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const found = () => !selector || document.querySelector(selector) !== null;
    const poll = () => {
      if (cancelled) return;
      if (found()) setDone(true);
      else timer = setTimeout(poll, 50);
    };
    (async () => {
      if (run) await run();
      poll();
    })();
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, []);
  return done ? null : <span data-story-pending="true" hidden />;
}
