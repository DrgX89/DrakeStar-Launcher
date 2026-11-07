export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const u = ['KiB', 'MiB', 'GiB', 'TiB'];
  let v = n / 1024, i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${u[i]}`;
}
export function when(ms: number | null): string {
  return ms ? new Date(ms).toLocaleString('es') : '—';
}
export function time(ms: number): string {
  return new Date(ms).toLocaleTimeString('es', { hour12: false });
}
export function slug(s: string): string {
  return s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 48);
}
