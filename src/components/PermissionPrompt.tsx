import { useState } from "react";

import { postDecision, type PendingEvent } from "../lib/tauri-client";
import { shortenCwd } from "../lib/time";
import { ToolIcon } from "./ToolIcon";

interface Props {
  pending: PendingEvent;
}

/**
 * Generic Allow/Deny prompt for a blocking PreToolUse event. Renders a
 * tool-specific body (diff for Edit, command for Bash, generic for
 * everything else) and exposes an optional reason input. Specialized
 * prompts (AskUserQuestion, ExitPlanMode) live in their own components
 * and are picked in Notch based on tool_name.
 */
export function PermissionPrompt({ pending }: Props) {
  const [reason, setReason] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(kind: "allow" | "deny") {
    if (submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      await postDecision(pending.event_id, {
        decision: kind,
        reason: reason.trim() || undefined,
      });
    } catch (e) {
      setError(String(e));
      setSubmitting(false);
    }
    // On success we leave submitting=true; the PendingResolved bus
    // message will unmount this component shortly.
  }

  return (
    <div className="prompt">
      <div className="prompt-header">
        <span className="prompt-tool">
          <ToolIcon tool={pending.tool_name} size={14} />
          {pending.tool_name ?? pending.event_type}
        </span>
        <span className="prompt-cwd" title={pending.cwd}>
          {shortenCwd(pending.cwd)}
        </span>
      </div>

      <div className="prompt-body">
        <ToolBody pending={pending} />
      </div>

      <input
        className="prompt-reason"
        placeholder="Reason (optional)"
        value={reason}
        onChange={(e) => setReason(e.target.value)}
        disabled={submitting}
      />

      {error && <div className="prompt-error">{error}</div>}

      <div className="prompt-actions">
        <button
          className="btn-deny"
          onClick={() => submit("deny")}
          disabled={submitting}
        >
          Deny
        </button>
        <button
          className="btn-allow"
          onClick={() => submit("allow")}
          disabled={submitting}
        >
          Allow
        </button>
      </div>
    </div>
  );
}

function ToolBody({ pending }: Props) {
  const input = (pending.tool_input ?? {}) as Record<string, unknown>;
  switch (pending.tool_name) {
    case "Edit":
      return <EditDiff input={input} />;
    case "MultiEdit":
      return <MultiEditDiff input={input} />;
    case "Write":
      return <WriteDiff input={input} />;
    case "Bash":
      return <BashCommand input={input} />;
    default:
      return <GenericTool input={input} />;
  }
}

function EditDiff({ input }: { input: Record<string, unknown> }) {
  const filePath = String(input.file_path ?? "");
  const oldStr = String(input.old_string ?? "");
  const newStr = String(input.new_string ?? "");
  return (
    <>
      <div className="prompt-filepath" title={filePath}>
        {filePath}
      </div>
      <div className="diff-pair">
        <pre className="diff-old">{oldStr || "(empty)"}</pre>
        <pre className="diff-new">{newStr || "(empty)"}</pre>
      </div>
    </>
  );
}

function MultiEditDiff({ input }: { input: Record<string, unknown> }) {
  const filePath = String(input.file_path ?? "");
  const edits = Array.isArray(input.edits)
    ? (input.edits as Array<Record<string, unknown>>)
    : [];
  return (
    <>
      <div className="prompt-filepath" title={filePath}>
        {filePath} · {edits.length} edit{edits.length === 1 ? "" : "s"}
      </div>
      <div className="diff-stack">
        {edits.slice(0, 3).map((e, i) => (
          <div key={i} className="diff-pair">
            <pre className="diff-old">{String(e.old_string ?? "")}</pre>
            <pre className="diff-new">{String(e.new_string ?? "")}</pre>
          </div>
        ))}
        {edits.length > 3 && (
          <div className="diff-more">+ {edits.length - 3} more edit(s)</div>
        )}
      </div>
    </>
  );
}

function WriteDiff({ input }: { input: Record<string, unknown> }) {
  const filePath = String(input.file_path ?? "");
  const content = String(input.content ?? "");
  return (
    <>
      <div className="prompt-filepath" title={filePath}>
        {filePath} · full write
      </div>
      <pre className="diff-new">{content.slice(0, 1000)}</pre>
      {content.length > 1000 && (
        <div className="diff-more">+ {content.length - 1000} bytes</div>
      )}
    </>
  );
}

function BashCommand({ input }: { input: Record<string, unknown> }) {
  const command = String(input.command ?? "");
  const description = input.description ? String(input.description) : null;
  return (
    <>
      {description && <div className="prompt-description">{description}</div>}
      <pre className="bash-command">{command}</pre>
    </>
  );
}

function GenericTool({ input }: { input: Record<string, unknown> }) {
  return (
    <pre className="tool-generic">{JSON.stringify(input, null, 2)}</pre>
  );
}
