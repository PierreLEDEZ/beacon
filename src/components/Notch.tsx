import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const COLLAPSE_DELAY_MS = 150;

export function Notch() {
  const [isExpanded, setIsExpanded] = useState(false);
  const collapseTimer = useRef<number | null>(null);

  // Push every state change down to the native window so the frame actually
  // grows/shrinks. The frontend CSS only affects layout inside the webview.
  useEffect(() => {
    invoke("resize_notch", { expanded: isExpanded }).catch((err) => {
      console.error("resize_notch failed", err);
    });
  }, [isExpanded]);

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

  // Small delay on leave avoids flicker when the cursor grazes the edge of
  // the collapsed pill (resize is async and the new window bounds may not
  // include the cursor anymore by the time the OS reports it).
  const handleMouseLeave = useCallback(() => {
    clearCollapseTimer();
    collapseTimer.current = window.setTimeout(() => {
      setIsExpanded(false);
    }, COLLAPSE_DELAY_MS);
  }, [clearCollapseTimer]);

  useEffect(() => clearCollapseTimer, [clearCollapseTimer]);

  return (
    <div
      className="notch-shell"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div className={`notch-core ${isExpanded ? "expanded" : "collapsed"}`}>
        {isExpanded ? (
          <div className="notch-expanded">
            <div className="notch-expanded-header">Beacon</div>
            <div className="notch-expanded-body">No active sessions yet.</div>
          </div>
        ) : (
          <span className="notch-label">Beacon</span>
        )}
      </div>
    </div>
  );
}
