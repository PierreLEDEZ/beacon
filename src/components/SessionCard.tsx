import { jumpToSession, type Session } from "../lib/tauri-client";
import { relTime, shortenCwd } from "../lib/time";
import { ToolIcon } from "./ToolIcon";

interface Props {
  session: Session;
}

export function SessionCard({ session }: Props) {
  const multiplexer = session.multiplexer?.kind ?? null;

  async function handleJump() {
    try {
      const report = await jumpToSession(session.claude_session_id);
      if (!report.focused_window && report.window_error) {
        console.warn("jump: window step failed", report.window_error);
      }
      if (report.multiplexer_error) {
        console.warn("jump: multiplexer step failed", report.multiplexer_error);
      }
    } catch (e) {
      console.error("jump failed", e);
    }
  }

  return (
    <button
      type="button"
      className="session-card"
      onClick={handleJump}
      title="Jump to this terminal"
    >
      <div className={`status-dot status-${session.status}`} />
      <div className="session-body">
        <div className="session-cwd" title={session.cwd}>
          {shortenCwd(session.cwd)}
        </div>
        <div className="session-meta">
          <span>{session.host_terminal.kind}</span>
          {multiplexer && <span> · {multiplexer}</span>}
          {session.last_tool_name && (
            <span className="session-tool">
              {" · "}
              <ToolIcon tool={session.last_tool_name} size={11} />
              {session.last_tool_name}
            </span>
          )}
          <span> · {relTime(session.last_activity)}</span>
        </div>
      </div>
    </button>
  );
}
