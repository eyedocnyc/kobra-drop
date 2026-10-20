// A stable manifest may only advance the installed version. The official Tauri
// plugin performs signature verification; this policy never bypasses it.
export function newerStable(current, candidate) {
  const parse = (v) =>
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(v)
      ? v.split(".").map(Number)
      : null;
  const a = parse(current),
    b = parse(candidate);
  if (!a || !b) return false;
  for (let i = 0; i < 3; i++) {
    if (b[i] !== a[i]) return b[i] > a[i];
  }
  return false;
}
// Installing exits the app, so a just-shown conversion result also defers it.
export function canInstall({
  converting,
  pending,
  selecting,
  installing,
  resultShown,
}) {
  return !converting && !pending && !selecting && !installing && !resultShown;
}
