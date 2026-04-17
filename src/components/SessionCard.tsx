import type { Session } from "../lib/tauri-client";
import { relTime, shortenCwd } from "../lib/time";

interface Props {
  session: Session;
}

export function SessionCard({ session }: Props) {
  const multiplexer = session.multiplexer?.kind ?? null;
  return (
    <div className="session-card">
      <div className={`status-dot status-${session.status}`} />
      <div className="session-body">
        <div className="session-cwd" title={session.cwd}>
          {shortenCwd(session.cwd)}
        </div>
        <div className="session-meta">
          <span>{session.host_terminal.kind}</span>
          {multiplexer && <span> · {multiplexer}</span>}
          {session.last_tool_name && (
            <span> · {session.last_tool_name}</span>
          )}
          <span> · {relTime(session.last_activity)}</span>
        </div>
      </div>
    </div>
  );
}
