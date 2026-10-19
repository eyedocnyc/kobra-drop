import type { Update, CheckOptions } from "@tauri-apps/plugin-updater";
export function fetchVerifiedUpdate(options: {
  configured: boolean;
  automatic: boolean;
  manual: boolean;
  currentVersion: string;
  checkUpdate: (options: CheckOptions) => Promise<Update | null>;
  status: (message: string) => void;
}): Promise<Update | null>;
