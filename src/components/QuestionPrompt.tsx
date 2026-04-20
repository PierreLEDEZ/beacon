import { useState } from "react";

import { postDecision, type PendingEvent } from "../lib/tauri-client";
import { shortenCwd } from "../lib/time";
import { ToolIcon } from "./ToolIcon";

interface Question {
  question: string;
  header?: string;
  options?: Array<{ label: string; description?: string }>;
  multiSelect?: boolean;
}

interface Props {
  pending: PendingEvent;
}

/**
 * Renders an AskUserQuestion tool call so the user answers directly
 * from Beacon.
 *
 * Mechanism: Claude Code's PreToolUse hook cannot synthesize a tool
 * result, but it can *block* a tool call with an arbitrary reason. When
 * Claude sees "AskUserQuestion was blocked. Reason: <user's answer>",
 * it reads the reason as the user's response and continues — same
 * technique used by Vibe Island for its agent-agnostic "Zero Config
 * hooks" support.
 *
 * Interactions:
 *   • click an option button  → block with reason = the selected label
 *   • "Free-text answer" expands an inline input for open answers
 *   • Cancel                  → block with a "declined" reason
 */
export function QuestionPrompt({ pending }: Props) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [freeTextFor, setFreeTextFor] = useState<number | null>(null);
  const [freeText, setFreeText] = useState("");

  const input = (pending.tool_input ?? {}) as Record<string, unknown>;
  const questions: Question[] = Array.isArray(input.questions)
    ? (input.questions as Question[])
    : [];

  /**
   * deny → hook writes {"decision":"block","reason":...} on stdout;
   * Claude reads the reason as the user's answer.
   */
  async function answer(questionIndex: number, value: string) {
    if (submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      await postDecision(pending.event_id, {
        decision: "deny",
        reason: `User answered Q${questionIndex + 1} via Beacon: "${value}"`,
      });
    } catch (e) {
      setError(String(e));
      setSubmitting(false);
    }
  }

  async function cancel() {
    if (submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      await postDecision(pending.event_id, {
        decision: "deny",
        reason: "User declined to answer via Beacon.",
      });
    } catch (e) {
      setError(String(e));
      setSubmitting(false);
    }
  }

  return (
    <div className="prompt">
      <div className="prompt-header">
        <span className="prompt-tool">
          <ToolIcon tool="AskUserQuestion" size={14} />
          AskUserQuestion
        </span>
        <span className="prompt-cwd" title={pending.cwd}>
          {shortenCwd(pending.cwd)}
        </span>
      </div>

      <div className="prompt-body">
        {questions.length === 0 && (
          <div className="prompt-description">(no question provided)</div>
        )}
        {questions.map((q, qi) => (
          <div key={qi} className="question-block">
            {q.header && <div className="question-header">{q.header}</div>}
            <div className="question-text">{q.question}</div>
            {q.options && q.options.length > 0 && (
              <div className="question-options">
                {q.options.map((opt, oi) => (
                  <button
                    key={oi}
                    className="question-option"
                    disabled={submitting}
                    onClick={() => answer(qi, opt.label)}
                    title={opt.description ?? undefined}
                  >
                    <span className="question-option-label">{opt.label}</span>
                    {opt.description && (
                      <span className="question-option-desc">
                        {opt.description}
                      </span>
                    )}
                  </button>
                ))}
              </div>
            )}

            {freeTextFor === qi ? (
              <div className="question-freetext-row">
                <input
                  className="prompt-reason"
                  placeholder="Type your answer…"
                  value={freeText}
                  onChange={(e) => setFreeText(e.target.value)}
                  autoFocus
                  disabled={submitting}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && freeText.trim()) {
                      answer(qi, freeText.trim());
                    } else if (e.key === "Escape") {
                      setFreeTextFor(null);
                      setFreeText("");
                    }
                  }}
                />
                <button
                  className="btn-allow"
                  disabled={!freeText.trim() || submitting}
                  onClick={() => answer(qi, freeText.trim())}
                >
                  Send
                </button>
              </div>
            ) : (
              <button
                className="question-freetext-toggle"
                disabled={submitting}
                onClick={() => {
                  setFreeTextFor(qi);
                  setFreeText("");
                }}
              >
                Free-text answer…
              </button>
            )}
          </div>
        ))}
      </div>

      {error && <div className="prompt-error">{error}</div>}

      <div className="prompt-actions">
        <button className="btn-deny" onClick={cancel} disabled={submitting}>
          Cancel
        </button>
      </div>
    </div>
  );
}
