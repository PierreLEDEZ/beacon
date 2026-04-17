import { useState } from "react";

import { postDecision, type PendingEvent } from "../lib/tauri-client";
import { shortenCwd } from "../lib/time";

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
 * Renders an AskUserQuestion tool call so the user sees the question
 * before approving. Clicking an option label is a shortcut that Allows
 * the tool call and records the chosen label in the reason field — the
 * actual answer will still flow through Claude's terminal-side UI once
 * the tool executes. Full answer-from-Beacon injection is Phase-4 work.
 */
export function QuestionPrompt({ pending }: Props) {
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const input = (pending.tool_input ?? {}) as Record<string, unknown>;
  const questions: Question[] = Array.isArray(input.questions)
    ? (input.questions as Question[])
    : [];

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
        <span className="prompt-tool">AskUserQuestion</span>
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
                    onClick={() =>
                      submit("allow", `Q${qi + 1} → ${opt.label}`)
                    }
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
          </div>
        ))}
      </div>

      {error && <div className="prompt-error">{error}</div>}

      <div className="prompt-actions">
        <button
          className="btn-deny"
          onClick={() => submit("deny", "User declined to answer via Beacon")}
          disabled={submitting}
        >
          Deny
        </button>
        <button
          className="btn-allow"
          onClick={() => submit("allow", undefined)}
          disabled={submitting}
        >
          Allow (answer in terminal)
        </button>
      </div>
    </div>
  );
}
