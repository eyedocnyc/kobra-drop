import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { canInstall } from "./update-policy.mjs";
import { fetchVerifiedUpdate } from "./update-service.mjs";

interface Preferences {
  profile_root: string | null;
  slicer_exe: string | null;
  automatic_updates: boolean | null;
}
interface Conversion {
  output: string;
  report: string;
  summary: string;
  warnings: string[];
  details: string;
}
let prefs: Preferences = {
  profile_root: null,
  slicer_exe: null,
  automatic_updates: true,
};
let pending: string | null = null,
  converting = false,
  selecting = false,
  installing = false,
  checking = false,
  ready: Update | null = null;
const app = document.querySelector<HTMLDivElement>("#app")!;
app.innerHTML = `<main><header><span class="mark" aria-hidden="true">K</span><div><h1>Kobra Drop</h1><p>MakerWorld projects, ready for your Kobra X.</p></div><span class="badge">0.4 mm</span></header>
<section id="drop" aria-label="Drop a 3MF project here"><span class="drop-icon" aria-hidden="true">⇩</span><h2>Drop your 3MF project here</h2><p>Or choose a downloaded MakerWorld / Bambu project.</p><button id="choose" class="secondary">Choose file</button><p id="filename">No file selected</p><button id="clear" class="quiet" hidden>Clear selection</button></section>
<button id="convert" class="primary" disabled>Convert for Kobra X</button><div id="status" role="status" aria-live="polite">Finding installed slicer profiles…</div><progress id="progress" hidden aria-label="Conversion in progress"></progress>
<section id="result" hidden><h2>Your project is ready</h2><p id="summary"></p><label>Saved beside your original</label><p id="output" class="path"></p><button id="open" class="primary">Open in Anycubic Slicer Next</button><p class="reminder">Choose your filament, slice, and check Preview.</p><p id="review"></p></section>
<details id="technical"><summary>Conversion details</summary><p id="report" class="path"></p><pre id="details">Your original file is never overwritten. Conversion stays on this computer.</pre></details>
<details id="settings"><summary>Settings</summary><p id="profiles"></p><div class="settings-row"><button id="profiles-dir" class="secondary">Locate profiles folder</button><button id="slicer-exe" class="secondary">Locate slicer</button></div><p id="locations" class="path"></p><label class="toggle"><input id="automatic" type="checkbox" checked> Automatically update when idle</label><button id="check-update" class="quiet">Check for updates</button><p id="update-status" role="status"></p></details><footer>Local conversion · No account needed</footer></main>`;
function el<T extends HTMLElement = HTMLElement>(id: string) {
  return document.getElementById(id) as T;
}
function status(message: string, error = false) {
  el("status").textContent = message;
  el("status").className = error ? "error" : "";
}
function updateStatus(message: string) {
  el("update-status").textContent = message;
}
function controls() {
  el<HTMLButtonElement>("choose").disabled =
    converting || selecting || installing;
  el<HTMLButtonElement>("convert").disabled =
    !pending || converting || selecting || installing;
  el<HTMLButtonElement>("clear").disabled = converting || installing;
  el("clear").hidden = !pending;
  el("filename").textContent =
    pending?.split(/[\\/]/).at(-1) ?? "No file selected";
  el("progress").hidden = !converting;
}
async function select(path: string | null) {
  if (converting || installing) return;
  selecting = true;
  controls();
  try {
    await invoke("select_file", { path });
    pending = path;
    el("result").hidden = true;
    status(path ? "Ready to convert." : "Choose a 3MF project.");
  } catch (e) {
    status(String(e), true);
  } finally {
    selecting = false;
    controls();
    void installWhenIdle();
  }
}
el("choose").onclick = async () => {
  selecting = true;
  controls();
  try {
    const file = await open({
      multiple: false,
      filters: [{ name: "3MF project", extensions: ["3mf"] }],
    });
    if (typeof file === "string") await select(file);
  } catch (e) {
    status(String(e), true);
  } finally {
    selecting = false;
    controls();
    void installWhenIdle();
  }
};
el("clear").onclick = async () => {
  await select(null);
  void installWhenIdle();
};
el("convert").onclick = async () => {
  if (!pending || converting || installing) return;
  converting = true;
  controls();
  el("result").hidden = true;
  try {
    const result = await invoke<Conversion>("convert_project");
    pending = null;
    el("summary").textContent = result.summary;
    el("output").textContent = result.output;
    el("report").textContent = `Report: ${result.report}`;
    el("details").textContent = result.details;
    el("review").textContent = result.warnings.join(" ");
    el("result").hidden = false;
    status("Converted and verified. Your original is unchanged.");
  } catch (e) {
    status(String(e), true);
  } finally {
    converting = false;
    controls();
    void installWhenIdle();
  }
};
el("open").onclick = async () => {
  try {
    await invoke("open_in_slicer");
    status("Opened in Anycubic Slicer Next. Import as a project.");
  } catch (e) {
    status(String(e), true);
    el<HTMLDetailsElement>("settings").open = true;
  }
};
async function save() {
  await invoke("save_preferences", { preferences: prefs });
}
async function discover() {
  const s = await invoke<{ found: boolean; message: string }>(
    "discover_profiles",
  );
  el("profiles").textContent = s.message;
  if (!s.found) {
    status(
      "Locate the installed slicer profiles in Settings before converting.",
      true,
    );
    el<HTMLDetailsElement>("settings").open = true;
  } else status(pending ? "Ready to convert." : "Choose a 3MF project.");
  el("locations").textContent = [prefs.profile_root, prefs.slicer_exe]
    .filter(Boolean)
    .join("\n");
}
el("profiles-dir").onclick = async () => {
  try {
    const p = await open({ directory: true, multiple: false });
    if (typeof p === "string") {
      prefs.profile_root = p;
      await save();
      await discover();
    }
  } catch (e) {
    status(String(e), true);
  }
};
el("slicer-exe").onclick = async () => {
  try {
    const p = await open({
      multiple: false,
      filters: [{ name: "AnycubicSlicerNext.exe", extensions: ["exe"] }],
    });
    if (typeof p === "string") {
      prefs.slicer_exe = p;
      await save();
      await discover();
    }
  } catch (e) {
    status(String(e), true);
  }
};
el("automatic").onchange = async () => {
  prefs.automatic_updates = el<HTMLInputElement>("automatic").checked;
  await save();
  if (prefs.automatic_updates) void checkUpdates();
};

async function installWhenIdle() {
  if (
    !ready ||
    prefs.automatic_updates === false ||
    !canInstall({ converting, pending: !!pending, selecting, installing })
  )
    return;
  installing = true;
  controls();
  try {
    if (!(await invoke<boolean>("reserve_update"))) {
      installing = false;
      controls();
      updateStatus(
        "Update ready. It will install after you finish with the selected file.",
      );
      return;
    }
    updateStatus("Installing verified update…");
    await ready.install();
    await relaunch();
  } catch (e) {
    await invoke("release_update");
    installing = false;
    ready = null;
    controls();
    updateStatus(
      `Update could not install. Conversion is available. ${String(e)}`,
    );
  }
}
async function checkUpdates(manual = false) {
  if (checking || installing) return;
  checking = true;
  try {
    if (ready) {
      await installWhenIdle();
      return;
    }
    const config = await invoke<{ configured: boolean }>("update_config");
    ready = await fetchVerifiedUpdate({
      configured: config.configured,
      automatic: prefs.automatic_updates !== false,
      manual,
      currentVersion: await getVersion(),
      checkUpdate: check,
      status: updateStatus,
    });
    if (!ready) return;
    if (manual && prefs.automatic_updates === false) {
      updateStatus(
        "Verified update downloaded. Enable automatic updates to install when idle.",
      );
    }
    await installWhenIdle();
  } catch {
    updateStatus("Update check unavailable. You can keep converting offline.");
  } finally {
    checking = false;
  }
}
el("check-update").onclick = () => void checkUpdates(true);
await listen<string>("conversion-progress", (event) => status(event.payload));
await listen<string>("open-file", (event) => {
  pending = event.payload;
  controls();
  status("Ready to convert.");
});
await listen<string>("file-waiting", (event) => status(event.payload, true));
await getCurrentWebview().onDragDropEvent((event) => {
  if (event.payload.type === "over") el("drop").classList.add("over");
  else el("drop").classList.remove("over");
  if (event.payload.type === "drop" && !selecting) {
    if (event.payload.paths.length !== 1) {
      status("Drop one 3MF project at a time.", true);
      return;
    }
    void select(event.payload.paths[0]);
  }
});
try {
  prefs = await invoke<Preferences>("get_preferences");
  pending = await invoke<string | null>("pending_file");
  el<HTMLInputElement>("automatic").checked = prefs.automatic_updates !== false;
  controls();
  await discover();
  void checkUpdates();
} catch (e) {
  status(`Could not initialize the desktop app: ${String(e)}`, true);
}
