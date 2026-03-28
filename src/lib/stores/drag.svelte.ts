/**
 * Pointer-event-based drag system that works in WKWebView.
 * HTML5 DnD is broken in Tauri's macOS webview — this replaces it entirely.
 */

export interface DragState {
  /** dir_path of the game being dragged */
  dirPath: string;
  /** Display label for the ghost */
  label: string;
  /** Current pointer position */
  x: number;
  y: number;
  /** Whether we've moved enough to start showing the ghost */
  active: boolean;
}

let dragState = $state<DragState | null>(null);
let dropTarget = $state<string | null>(null);

const DRAG_THRESHOLD = 5; // px before drag activates
let startX = 0;
let startY = 0;

export function getDragState(): DragState | null {
  return dragState;
}

export function getDropTarget(): string | null {
  return dropTarget;
}

export function setDropTarget(target: string | null): void {
  dropTarget = target;
}

export function startDrag(dirPath: string, label: string, x: number, y: number): void {
  startX = x;
  startY = y;
  dragState = { dirPath, label, x, y, active: false };
}

export function moveDrag(x: number, y: number): void {
  if (!dragState) return;
  const dx = x - startX;
  const dy = y - startY;
  const active = dragState.active || Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD;
  dragState = { ...dragState, x, y, active };
}

export function endDrag(): { dirPath: string; tournament: string | null } | null {
  if (!dragState) return null;
  const result = dragState.active && dropTarget
    ? { dirPath: dragState.dirPath, tournament: dropTarget }
    : null;
  dragState = null;
  dropTarget = null;
  return result;
}

export function cancelDrag(): void {
  dragState = null;
  dropTarget = null;
}

export function isDragging(): boolean {
  return dragState?.active ?? false;
}

export function getDraggingGameDir(): string | null {
  return dragState?.dirPath ?? null;
}
