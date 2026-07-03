export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  return `${m}:${String(sec).padStart(2, "0")}`;
}

export function formatDurationLong(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0 && m > 0) return `${h}h ${m}m`;
  if (h > 0) return `${h}h`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}

export function formatDateTime(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString();
}

export function formatDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString();
}

export function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

export function startOfWeekUnix(d = new Date()): number {
  const day = d.getDay();
  const diff = (day + 6) % 7; // Monday start
  const monday = new Date(d.getFullYear(), d.getMonth(), d.getDate() - diff);
  return Math.floor(monday.getTime() / 1000);
}

export function startOfMonthUnix(d = new Date()): number {
  const first = new Date(d.getFullYear(), d.getMonth(), 1);
  return Math.floor(first.getTime() / 1000);
}

export function addDays(unixSeconds: number, days: number): number {
  return unixSeconds + days * 86400;
}
