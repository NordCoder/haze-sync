import type { ConflictResolutionAction } from "./api-client";
import { generateConflictResolutionIdempotencyKey } from "./idempotency-keys";

export interface ConflictActionKeyEntry {
  serverContext: string;
  conflictId: string;
  action: ConflictResolutionAction;
  idempotencyKey: string;
  createdAt: string;
}

export interface ConflictActionKeyState {
  byAction: Record<string, ConflictActionKeyEntry>;
}

export interface PreparedConflictActionKey {
  state: ConflictActionKeyState;
  idempotencyKey: string;
}

export function createDefaultConflictActionKeyState(): ConflictActionKeyState {
  return { byAction: {} };
}

export function mergeConflictActionKeyState(rawState: unknown): ConflictActionKeyState {
  if (!isRecord(rawState) || !isRecord(rawState.byAction)) {
    return createDefaultConflictActionKeyState();
  }

  const byAction: Record<string, ConflictActionKeyEntry> = {};
  for (const [key, value] of Object.entries(rawState.byAction)) {
    if (isConflictActionKeyEntry(value)) {
      byAction[key] = value;
    }
  }

  return { byAction };
}

export function prepareConflictActionKey(
  state: ConflictActionKeyState,
  serverContext: string,
  conflictId: string,
  action: ConflictResolutionAction,
  observedAt: string,
): PreparedConflictActionKey {
  const mapKey = actionMapKey(serverContext, conflictId, action);
  const existing = state.byAction[mapKey];
  if (existing !== undefined) {
    return {
      state,
      idempotencyKey: existing.idempotencyKey,
    };
  }

  const entry: ConflictActionKeyEntry = {
    serverContext,
    conflictId,
    action,
    idempotencyKey: generateConflictResolutionIdempotencyKey(action),
    createdAt: observedAt,
  };

  return {
    state: {
      byAction: {
        ...state.byAction,
        [mapKey]: entry,
      },
    },
    idempotencyKey: entry.idempotencyKey,
  };
}

export function clearConflictActionKey(
  state: ConflictActionKeyState,
  serverContext: string,
  conflictId: string,
  action: ConflictResolutionAction,
): ConflictActionKeyState {
  const mapKey = actionMapKey(serverContext, conflictId, action);
  if (state.byAction[mapKey] === undefined) {
    return state;
  }

  const byAction = { ...state.byAction };
  delete byAction[mapKey];
  return { byAction };
}

export function retainOpenConflictActionKeys(
  state: ConflictActionKeyState,
  serverContext: string,
  openConflictIds: ReadonlySet<string>,
): ConflictActionKeyState {
  const retained: Record<string, ConflictActionKeyEntry> = {};
  let changed = false;

  for (const [key, entry] of Object.entries(state.byAction)) {
    if (entry.serverContext === serverContext && openConflictIds.has(entry.conflictId)) {
      retained[key] = entry;
    } else {
      changed = true;
    }
  }

  return changed ? { byAction: retained } : state;
}

export function conflictServerContext(serverUrl: string, adapterId: string): string {
  return `${serverUrl}\n${adapterId}`;
}

function actionMapKey(serverContext: string, conflictId: string, action: ConflictResolutionAction): string {
  return `${encodeURIComponent(serverContext)}:${encodeURIComponent(conflictId)}:${action}`;
}

function isConflictActionKeyEntry(value: unknown): value is ConflictActionKeyEntry {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.serverContext === "string" &&
    typeof value.conflictId === "string" &&
    isConflictResolutionAction(value.action) &&
    typeof value.idempotencyKey === "string" &&
    value.idempotencyKey.length > 0 &&
    typeof value.createdAt === "string"
  );
}

function isConflictResolutionAction(value: unknown): value is ConflictResolutionAction {
  return (
    value === "accept_current" ||
    value === "accept_conflict" ||
    value === "keep_both" ||
    value === "mark_resolved"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
