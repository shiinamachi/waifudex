const DRAG_LOCK_EVENT_TYPES = [
  "dragstart",
  "dragenter",
  "dragover",
  "drop",
] as const;

export function isTextSelectionAllowedTarget(
  target: EventTarget | null,
): boolean {
  if (!(target instanceof Node)) {
    return false;
  }

  const element =
    target instanceof Element ? target : target.parentElement;

  if (!element) {
    return false;
  }

  return element.closest('input, textarea, [contenteditable="true"]') !== null;
}

export function installGlobalDragLock(document: Document): () => void {
  const handleDragEvent = (event: Event): void => {
    event.preventDefault();
  };

  for (const eventType of DRAG_LOCK_EVENT_TYPES) {
    document.addEventListener(eventType, handleDragEvent, true);
  }

  return () => {
    for (const eventType of DRAG_LOCK_EVENT_TYPES) {
      document.removeEventListener(eventType, handleDragEvent, true);
    }
  };
}
