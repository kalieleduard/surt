import { useEffect, useRef, useState } from "react";

const listeners = new Set<(down: boolean) => void>();
let pressed = false;

function update(event: KeyboardEvent) {
  if (event.shiftKey === pressed) return;
  pressed = event.shiftKey;
  for (const listener of listeners) listener(pressed);
}

function release() {
  if (!pressed) return;
  pressed = false;
  for (const listener of listeners) listener(false);
}

function subscribe(listener: (down: boolean) => void) {
  if (!listeners.size) {
    window.addEventListener("keydown", update);
    window.addEventListener("keyup", update);
    window.addEventListener("blur", release);
  }
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (listeners.size) return;
    window.removeEventListener("keydown", update);
    window.removeEventListener("keyup", update);
    window.removeEventListener("blur", release);
  };
}

export function useShiftKey(): boolean {
  const [down, setDown] = useState(pressed);
  useEffect(() => subscribe(setDown), []);
  return down;
}

export function useShiftRef() {
  const down = useRef(pressed);
  useEffect(
    () =>
      subscribe((value) => {
        down.current = value;
      }),
    [],
  );
  return down;
}
