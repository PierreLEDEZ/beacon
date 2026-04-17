import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";

import {
  listPending,
  listSessions,
  onBusMessage,
  resizeNotch,
  type PendingEvent,
  type Session,
} from "../lib/tauri-client";
import { PermissionPrompt } from "./PermissionPrompt";
import { SessionCard } from "./SessionCard";

const COLLAPSE_DELAY_MS = 150;
const NOW_TICK_MS = 5_000;

type GlobalStatus =
  | "idle"
  | "working"
  | "waiting_approval"
  | "error"
  | "empty";

function globalStatus(
  sessions: Session[],
  pendings: PendingEvent[],
): GlobalStatus {
  if (pendings.length > 0) return "waiting_approval";
  if (sessions.length === 0) return "empty";
  if (sessions.some((s) => s.status === "error")) return "error";
  if (sessions.some((s) => s.status === "working")) return "working";
  return "idle";
}

export function Notch() {
  const [isExpanded, setIsExpanded] = useState(false);
  const [sessions, setSessions] = useState<Map<string, Session>>(new Map());
  const [pendings, setPendings] = useState<Map<string, PendingEvent>>(
    new Map(),
  );
  const [, setTick] = useState(0);
  const collapseTimer = useRef<number | null>(null);

  // When a prompt shows up the notch forces-expands so the user sees it.
  // Hover still works; user-initiated collapse is vetoed while anything is
  // pending.
  const pendingList = useMemo(
    () =>
      Array.from(pendings.values()).sort(
        (a, b) =>
          new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
      ),
    [pendings],
  );
  const hasPending = pendingList.length > 0;
  const effectiveExpanded = hasPending || isExpanded;

  useEffect(() => {
    resizeNotch(effectiveExpanded).catch((err) => {
      console.error("resize_notch failed", err);
    });
  }, [effectiveExpanded]);

  useEffect(() => {
    let unlistenFn: UnlistenFn | null = null;
    let cancelled = false;

    (async () => {
      try {
        const [initialSessions, initialPending] = await Promise.all([
          listSessions(),
          listPending(),
        ]);
        if (cancelled) return;
        setSessions(
          new Map(initialSessions.map((s) => [s.claude_session_id, s])),
        );
        setPendings(new Map(initialPending.map((p) => [p.event_id, p])));
      } catch (err) {
        console.error("initial hydrate failed", err);
      }

      unlistenFn = await onBusMessage((msg) => {
        if (msg.kind === "session_updated") {
          setSessions((prev) => {
            const next = new Map(prev);
            next.set(msg.session.claude_session_id, msg.session);
            return next;
          });
        } else if (msg.kind === "session_removed") {
          setSessions((prev) => {
            const next = new Map(prev);
            next.delete(msg.claude_session_id);
            return next;
          });
        } else if (msg.kind === "pending_awaiting") {
          setPendings((prev) => {
            const next = new Map(prev);
            next.set(msg.pending.event_id, msg.pending);
            return next;
          });
        } else if (msg.kind === "pending_resolved") {
          setPendings((prev) => {
            const next = new Map(prev);
            next.delete(msg.event_id);
            return next;
          });
        }
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
  const gStatus = globalStatus(sessionList, pendingList);
  const currentPending = pendingList[0] ?? null;

  return (
    <div
      className="notch-shell"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div
        className={`notch-core ${effectiveExpanded ? "expanded" : "collapsed"}`}
      >
        {effectiveExpanded ? (
          <div className="notch-expanded">
            <div className="notch-expanded-header">
              {hasPending ? (
                <>
                  <span>Awaiting approval</span>
                  <span className="notch-count">
                    {pendingList.length > 1
                      ? `1 / ${pendingList.length}`
                      : pendingList.length}
                  </span>
                </>
              ) : (
                <>
                  <span>Beacon</span>
                  <span className="notch-count">{sessionList.length}</span>
                </>
              )}
            </div>
            <div className="notch-expanded-body">
              {currentPending ? (
                <PermissionPrompt
                  key={currentPending.event_id}
                  pending={currentPending}
                />
              ) : sessionList.length === 0 ? (
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
            <span className="notch-count">
              {hasPending ? pendingList.length : sessionList.length}
            </span>
            <span className="notch-label">
              {hasPending ? "Pending" : "Beacon"}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
