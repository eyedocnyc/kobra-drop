import test from "node:test";
import assert from "node:assert/strict";
import { fetchVerifiedUpdate } from "../src/update-service.mjs";
const defaults = {
  configured: true,
  automatic: true,
  manual: false,
  currentVersion: "0.1.0",
  status: () => {},
};
test("unconfigured and opted-out startup never contact an endpoint", async () => {
  let calls = 0;
  const checkUpdate = async () => {
    calls++;
    throw Error("network must not be used");
  };
  assert.equal(
    await fetchVerifiedUpdate({ ...defaults, configured: false, checkUpdate }),
    null,
  );
  assert.equal(
    await fetchVerifiedUpdate({ ...defaults, automatic: false, checkUpdate }),
    null,
  );
  assert.equal(calls, 0);
});
test("offline startup resolves without blocking conversion and has bounded timeout", async () => {
  let message = "";
  const result = await fetchVerifiedUpdate({
    ...defaults,
    status: (s) => (message = s),
    checkUpdate: async (options) => {
      assert.equal(options.timeout, 8000);
      throw Error("offline");
    },
  });
  assert.equal(result, null);
  assert.match(message, /keep converting offline/);
});
test("failed official signature verification never returns an installable update", async () => {
  let closed = false;
  const update = {
    version: "0.2.0",
    download: async () => {
      throw Error("invalid signature");
    },
    close: async () => {
      closed = true;
    },
  };
  assert.equal(
    await fetchVerifiedUpdate({ ...defaults, checkUpdate: async () => update }),
    null,
  );
  assert.equal(closed, true);
});
test("only newer stable releases download; verification completes before ready", async () => {
  for (const version of ["0.0.9", "0.1.0", "0.2.0-beta.1"]) {
    let downloaded = false;
    await fetchVerifiedUpdate({
      ...defaults,
      checkUpdate: async () => ({
        version,
        download: async () => {
          downloaded = true;
        },
        close: async () => {},
      }),
    });
    assert.equal(downloaded, false);
  }
  let complete = false;
  const update = {
    version: "0.2.0",
    download: async (_, options) => {
      // Tauri's timeout bounds the whole request. The NSIS update bundles the
      // offline WebView2 installer (~210 MiB); allow 2 Mbit/s (~880 s).
      assert.ok(options.timeout >= 900000, `download timeout ${options.timeout} ms`);
      complete = true;
    },
    close: async () => {},
  };
  assert.equal(
    await fetchVerifiedUpdate({ ...defaults, checkUpdate: async () => update }),
    update,
  );
  assert.equal(complete, true);
});
