// Small fuzzy matcher for file names and commands: every query character in order, with
// bonuses for matches in the file name, at word starts and in a row.

export function score(query: string, text: string): number {
  if (!query) return 1;
  const q = query.toLowerCase();
  const s = text.toLowerCase();
  const nameStart = s.lastIndexOf("/") + 1;
  let qi = 0;
  let total = 0;
  let run = 0;
  for (let i = 0; i < s.length && qi < q.length; i++) {
    if (s[i] !== q[qi]) {
      run = 0;
      continue;
    }
    let pts = 1;
    if (i >= nameStart) pts += 2;
    if (i === 0 || "/._- ".includes(s[i - 1])) pts += 3;
    run++;
    pts += run;
    total += pts;
    qi++;
  }
  if (qi < q.length) return 0;
  // Exact name and prefix matches first, then shorter paths.
  const name = s.slice(nameStart);
  if (name === q) total += 100;
  else if (name.startsWith(q)) total += 30;
  return total - s.length * 0.01;
}

export function filter<T>(items: T[], query: string, key: (t: T) => string, limit = 50): T[] {
  if (!query) return items.slice(0, limit);
  const scored: [number, T][] = [];
  for (const it of items) {
    const sc = score(query, key(it));
    if (sc > 0) scored.push([sc, it]);
  }
  scored.sort((a, b) => b[0] - a[0]);
  return scored.slice(0, limit).map((x) => x[1]);
}
