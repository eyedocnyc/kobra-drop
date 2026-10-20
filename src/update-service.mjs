import { newerStable } from "./update-policy.mjs";
// Tauri applies this to the whole download, not to idle time. The NSIS update
// bundles the offline WebView2 installer (~210 MiB), so allow slow connections.
export const DOWNLOAD_TIMEOUT_MS = 30 * 60 * 1000;
/** The desktop adapter supplies the official Tauri updater. A rejected download
 * (including signature failure) can never become an installable result. */
export async function fetchVerifiedUpdate({
  configured,
  automatic,
  manual,
  currentVersion,
  checkUpdate,
  status,
}) {
  if (!configured) {
    status("Updates are not configured for this development build.");
    return null;
  }
  if (!manual && !automatic) {
    status("Automatic updates are off.");
    return null;
  }
  let update;
  try {
    status("Checking for updates…");
    update = await checkUpdate({ timeout: 8000 });
    if (!update || !newerStable(currentVersion, update.version)) {
      await update?.close();
      status("You have the latest stable version.");
      return null;
    }
    status(`Downloading ${update.version}…`);
    let bytes = 0;
    await update.download(
      (event) => {
        if (event.event === "Progress") {
          bytes += event.data.chunkLength;
          status(`Downloading update… ${(bytes / 1048576).toFixed(1)} MB`);
        }
      },
      { timeout: DOWNLOAD_TIMEOUT_MS },
    );
    status(
      "Verified update ready. Installation waits until no file is pending.",
    );
    return update;
  } catch {
    try {
      await update?.close();
    } catch {
      /* cleanup must not block conversion */
    }
    status(
      "Update unavailable or verification failed. You can keep converting offline.",
    );
    return null;
  }
}
