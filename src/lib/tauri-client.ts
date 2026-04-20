import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Status =
  | "idle"
  | "working"
  | "waiting_approval"
  | "done"
  | "error";

export interface MultiplexerLocation {
  kind: string;
  session?: string;
  tab?: string;
  pane?: string;
}

export interface HostTerminal {
  kind: string;
  markers?: unknown;
}

export interface Session {
  claude_session_id: string;
  first_seen: string;
  last_activity: string;
  status: Status;
  cwd: string;
  multiplexer?: MultiplexerLocation;
  host_terminal: HostTerminal;
  last_event_type?: string;
  last_tool_name?: string;
}

export interface PendingEvent {
  event_id: string;
  session_id: string;
  event_type: string;
  cwd: string;
  tool_name?: string;
  tool_input?: Record<string, unknown>;
  created_at: string;
}

export type DecisionKind = "allow" | "deny" | "answer";

export interface Decision {
  decision: DecisionKind;
  reason?: string;
  answer?: string;
}

export type BusMessage =
  | { kind: "session_updated"; session: Session }
  | { kind: "session_removed"; claude_session_id: string }
  | { kind: "pending_awaiting"; pending: PendingEvent }
  | { kind: "pending_resolved"; event_id: string; decision: Decision };

export const BUS_EVENT = "beacon://bus";
export const SHORTCUT_EVENT = "beacon://shortcut";

export type ShortcutAction =
  | { action: "allow_top_pending" }
  | { action: "deny_top_pending" };

export function listSessions(): Promise<Session[]> {
  return invoke("list_sessions");
}

export function listPending(): Promise<PendingEvent[]> {
  return invoke("list_pending");
}

export interface JumpReport {
  focused_window: boolean;
  focused_pane: boolean;
  window_error?: string;
  multiplexer_error?: string;
}

export function jumpToSession(
  claudeSessionId: string,
): Promise<JumpReport> {
  return invoke("jump_session", { claudeSessionId });
}

export function resizeNotch(expanded: boolean): Promise<void> {
  return invoke("resize_notch", { expanded });
}

/**
 * Resolve a pending prompt with the user's Allow/Deny/Answer.
 *
 * Uses Tauri IPC (invoke) to bypass CORS on the webview's origin. The
 * backend fulfills the same one-shot channel the HTTP /decision/:id
 * route would, so the hook's long-poll sees the decision regardless of
 * which path resolved it.
 */
export function postDecision(
  eventId: string,
  decision: Decision,
): Promise<void> {
  return invoke("resolve_pending", { eventId, decision });
}

export function onBusMessage(
  callback: (msg: BusMessage) => void,
): Promise<UnlistenFn> {
  return listen<BusMessage>(BUS_EVENT, (event) => callback(event.payload));
}

export function onShortcutAction(
  callback: (msg: ShortcutAction) => void,
): Promise<UnlistenFn> {
  return listen<ShortcutAction>(SHORTCUT_EVENT, (event) =>
    callback(event.payload),
  );
}
