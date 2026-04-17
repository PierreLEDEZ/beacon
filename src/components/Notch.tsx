import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";

import {
  listSessions,
  onBusMessage,
  resizeNotch,
  type Session,
} from "../lib/tauri-client";
import { SessionCard } from "./SessionCard";

const COLLAPSE_DELAY_MS = 150;
const NOW_TICK_MS = 5_000;

type GlobalStatus = "idle" | "working" | "error" | "empty";

function globalStatus(sessions: Session[]): GlobalStatus {
  if (sessions.length === 0) return "empty";
  if (sessions.some((s) => s.status === "error")) return "error";
  if (sessions.some((s) => s.status === "working")) return "working";
  return "idle";
}

export function Notch() {
  const [isExpanded, setIsExpanded] = useState(false);
  const [sessions, setSessions] = useState<Map<string, Session>>(new Map());
  // Ticker so relative times ("3s ago") update without a dedicated per-card
  // interval — bumping this key re-renders every SessionCard once.
  const [, setTick] = useState(0);
  const collapseTimer = useRef<number | null>(null);

  useEffect(() => {
    resizeNotch(isExpanded).catch((err) => {
      console.error("resize_notch failed", err);
    });
  }, [isExpanded]);

  useEffect(() => {
    let unlistenFn: UnlistenFn | null = null;
    let cancelled = false;

    (async () => {
      try {
        const initial = await listSessions();
        if (cancelled) return;
        setSessions(new Map(initial.map((s) => [s.claude_session_id, s])));
      } catch (err) {
        console.error("listSessions failed", err);
      }

      unlistenFn = await onBusMessage((msg) => {
        setSessions((prev) => {
          const next = new Map(prev);
          if (msg.kind === "session_updated") {
            next.set(msg.session.claude_session_id, msg.session);
          } else if (msg.kind === "session_removed") {
            next.delete(msg.claude_session_id);
          }
          return next;
        });
      });
      if (cancelled) unlistenFn?.();
    })();

    return () => {
      cancelled = true;
      unlistenFn?.();
    };
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => setTick((t) => t + 1), NOW_TICK_MS);
    return () => window.clearInterval(id);
  }, []);

  const clearCollapseTimer = useCallback(() => {
    if (collapseTimer.current !== null) {
      window.clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }
  }, []);

  const handleMouseEnter = useCallback(() => {
    clearCollapseTimer();
    setIsExpanded(true);
  }, [clearCollapseTimer]);

  const handleMouseLeave = useCallback(() => {
    clearCollapseTimer();
    collapseTimer.current = window.setTimeout(() => {
      setIsExpanded(false);
    }, COLLAPSE_DELAY_MS);
  }, [clearCollapseTimer]);

  useEffect(() => clearCollapseTimer, [clearCollapseTimer]);

  const sessionList = useMemo(
    () =>
      Array.from(sessions.values()).sort(
        (a, b) =>
          new Date(b.last_activity).getTime() -
          new Date(a.last_activity).getTime(),
      ),
    [sessions],
  );
  const gStatus = globalStatus(sessionList);

  return (
    <div
      className="notch-shell"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div className={`notch-core ${isExpanded ? "expanded" : "collapsed"}`}>
        {isExpanded ? (
          <div className="notch-expanded">
            <div className="notch-expanded-header">
              <span>Beacon</span>
              <span className="notch-count">{sessionList.length}</span>
            </div>
            <div className="notch-expanded-body">
              {sessionList.length === 0 ? (
                <div className="notch-empty">No active sessions yet.</div>
              ) : (
                sessionList.map((s) => (
                  <SessionCard key={s.claude_session_id} session={s} />
                ))
              )}
            </div>
          </div>
        ) : (
          <div className="notch-collapsed-row">
            <div className={`status-dot status-${gStatus}`} />
            <span className="notch-count">{sessionList.length}</span>
            <span className="notch-label">Beacon</span>
          </div>
        )}
      </div>
    </div>
  );
}
