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

export function listSessions(): Promise<Session[]> {
  return invoke("list_sessions");
}

export function listPending(): Promise<PendingEvent[]> {
  return invoke("list_pending");
}

export function resizeNotch(expanded: boolean): Promise<void> {
  return invoke("resize_notch", { expanded });
}

/**
 * POST the user's Allow/Deny (or Answer) to the local Beacon server.
 * Uses fetch instead of invoke since the decision goes via HTTP — the
 * hook is waiting on /wait/:id, so the resolution must travel through
 * the same axum router.
 */
export async function postDecision(
  eventId: string,
  decision: Decision,
): Promise<void> {
  const res = await fetch(`http://127.0.0.1:37421/decision/${eventId}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(decision),
  });
  if (!res.ok && res.status !== 204) {
    throw new Error(`decision POST failed: ${res.status} ${res.statusText}`);
  }
}

export function onBusMessage(
  callback: (msg: BusMessage) => void,
): Promise<UnlistenFn> {
  return listen<BusMessage>(BUS_EVENT, (event) => callback(event.payload));
}
