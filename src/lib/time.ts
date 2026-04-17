export function shortenCwd(cwd: string, max = 38): string {
  if (cwd.length <= max) return cwd;
  const parts = cwd.split(/[\\/]/);
  if (parts.length <= 2) return cwd.slice(-max);
  return `…/${parts.slice(-2).join("/")}`;
}

export function relTime(iso: string, now = Date.now()): string {
  const t = new Date(iso).getTime();
  const s = Math.max(0, Math.round((now - t) / 1000));
  if (s < 5) return "now";
  if (s < 60) return `${s}s ago`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.round(h / 24);
  return `${d}d ago`;
}
