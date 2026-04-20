/**
 * Small inline SVG icon per Claude Code tool. Hand-rolled instead of
 * pulling in lucide-react (~50kb) because we need ~8 icons total.
 * 14x14 default, inherits currentColor — the caller wraps it in
 * whatever color/opacity context fits the surrounding UI.
 */

interface Props {
  tool?: string | null;
  size?: number;
  className?: string;
}

export function ToolIcon({ tool, size = 14, className }: Props) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    className,
  };

  switch (tool) {
    case "Edit":
    case "MultiEdit":
    case "Write":
    case "NotebookEdit":
      // pencil
      return (
        <svg {...common}>
          <path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
        </svg>
      );
    case "Read":
      // eye
      return (
        <svg {...common}>
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8S1 12 1 12Z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      );
    case "Bash":
      // terminal chevron
      return (
        <svg {...common}>
          <polyline points="4 17 10 11 4 5" />
          <line x1="12" y1="19" x2="20" y2="19" />
        </svg>
      );
    case "Grep":
      // magnifier
      return (
        <svg {...common}>
          <circle cx="11" cy="11" r="7" />
          <line x1="21" y1="21" x2="16.5" y2="16.5" />
        </svg>
      );
    case "Glob":
      // asterisk-ish
      return (
        <svg {...common}>
          <line x1="12" y1="3" x2="12" y2="21" />
          <line x1="5.6" y1="5.6" x2="18.4" y2="18.4" />
          <line x1="5.6" y1="18.4" x2="18.4" y2="5.6" />
        </svg>
      );
    case "AskUserQuestion":
      // question mark in circle
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="10" />
          <path d="M9.09 9a3 3 0 0 1 5.82 1c0 2-3 3-3 3" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
      );
    case "ExitPlanMode":
      // clipboard with check
      return (
        <svg {...common}>
          <rect x="8" y="3" width="8" height="4" rx="1" />
          <path d="M8 5H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2" />
          <polyline points="9 14 11 16 15 12" />
        </svg>
      );
    case "Task":
      // sparkle/layers
      return (
        <svg {...common}>
          <polygon points="12 2 15 9 22 9 17 14 19 22 12 18 5 22 7 14 2 9 9 9" />
        </svg>
      );
    case "WebFetch":
    case "WebSearch":
      // globe
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="10" />
          <line x1="2" y1="12" x2="22" y2="12" />
          <path d="M12 2a15 15 0 0 1 0 20M12 2a15 15 0 0 0 0 20" />
        </svg>
      );
    case "TodoWrite":
      // list
      return (
        <svg {...common}>
          <line x1="8" y1="6" x2="21" y2="6" />
          <line x1="8" y1="12" x2="21" y2="12" />
          <line x1="8" y1="18" x2="21" y2="18" />
          <line x1="3" y1="6" x2="3.01" y2="6" />
          <line x1="3" y1="12" x2="3.01" y2="12" />
          <line x1="3" y1="18" x2="3.01" y2="18" />
        </svg>
      );
    default:
      // wrench fallback
      return (
        <svg {...common}>
          <path d="M14.7 6.3a4 4 0 0 0 5 5L22 14l-8 8-8-8L2.3 11.3a4 4 0 0 0 5-5Z" />
        </svg>
      );
  }
}
