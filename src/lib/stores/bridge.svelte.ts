import type { Readable } from "svelte/store";
import { onDestroy } from "svelte";

/**
 * Bridge a Svelte store into reactive $state via a class field.
 * $state in class fields IS supported in .svelte.ts; $state in plain functions is NOT.
 */
class ReactiveBox<T> {
  current = $state<T>(undefined as T);
}

export function useStore<T>(store: Readable<T>): () => T {
  const box = new ReactiveBox<T>();
  const unsub = store.subscribe((v) => {
    box.current = v;
  });
  onDestroy(unsub);
  return () => box.current;
}
