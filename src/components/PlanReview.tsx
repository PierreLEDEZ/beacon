import { useMemo, useState } from "react";
import { marked } from "marked";

import { postDecision, type PendingEvent } from "../lib/tauri-client";
import { shortenCwd } from "../lib/time";

interface Props {
  pending: PendingEvent;
}

/**
 * ExitPlanMode renderer. Shows the plan Markdown; user can Approve (→
 * allow) or send Feedback (→ deny with reason = user's message, which
 * Claude will read as context and respond to with a revised plan).
 */
export function PlanReview({ pending }: Props) {
  const [feedback, setFeedback] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const input = (pending.tool_input ?? {}) as Record<string, unknown>;
  const plan = typeof input.plan === "string" ? input.plan : "";

  // marked.parse returns string | Promise<string>; we run it sync here.
  const html = useMemo(() => {
    if (!plan) return "";
    try {
      return marked.parse(plan, { async: false }) as string;
    } catch (e) {
      console.error("plan markdown parse failed", e);
      return `<pre>${escapeHtml(plan)}</pre>`;
    }
  }, [plan]);

  async function submit(
    kind: "allow" | "deny",
    reason: string | undefined,
  ) {
    if (submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      await postDecision(pending.event_id, {
        decision: kind,
        reason: reason?.trim() || undefined,
      });
    } catch (e) {
      setError(String(e));
      setSubmitting(false);
    }
  }

  return (
    <div className="prompt">
      <div className="prompt-header">
        <span className="prompt-tool">ExitPlanMode</span>
        <span className="prompt-cwd" title={pending.cwd}>
          {shortenCwd(pending.cwd)}
        </span>
      </div>

      <div className="prompt-body">
        {plan ? (
          <div
            className="plan-markdown"
            // Content comes from Claude on the local machine — no
            // cross-origin concerns. We trust the author of the plan.
            dangerouslySetInnerHTML={{ __html: html }}
          />
        ) : (
          <div className="prompt-description">(empty plan)</div>
        )}
      </div>

      <textarea
        className="prompt-feedback"
        placeholder="Feedback to send back to Claude (optional for Approve, required for Feedback)"
        value={feedback}
        onChange={(e) => setFeedback(e.target.value)}
        disabled={submitting}
        rows={2}
      />

      {error && <div className="prompt-error">{error}</div>}

      <div className="prompt-actions">
        <button
          className="btn-deny"
          onClick={() =>
            submit(
              "deny",
              feedback.trim() || "User declined the plan via Beacon",
            )
          }
          disabled={submitting}
        >
          Feedback
        </button>
        <button
          className="btn-allow"
          onClick={() => submit("allow", feedback.trim() || undefined)}
          disabled={submitting}
        >
          Approve
        </button>
      </div>
    </div>
  );
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
