import { isTauri } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";

export async function confirmAction(message: string, title: string, okLabel: string): Promise<boolean> {
  if (!isTauri()) return window.confirm(message);
  return ask(message, { title, kind: "warning", okLabel, cancelLabel: "Cancel" });
}
