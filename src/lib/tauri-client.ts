import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Status = "idle" | "working" | "done" | "error";

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

export type BusMessage =
  | { kind: "session_updated"; session: Session }
  | { kind: "session_removed"; claude_session_id: string };

export const BUS_EVENT = "beacon://bus";

export function listSessions(): Promise<Session[]> {
  return invoke("list_sessions");
}

export function resizeNotch(expanded: boolean): Promise<void> {
  return invoke("resize_notch", { expanded });
}

export function onBusMessage(
  callback: (msg: BusMessage) => void,
): Promise<UnlistenFn> {
  return listen<BusMessage>(BUS_EVENT, (event) => callback(event.payload));
}
