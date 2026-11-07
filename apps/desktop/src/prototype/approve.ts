import { invoke, isTauri } from "@tauri-apps/api/core";
import { describeChoice, type Choice } from "./options";

const STORAGE_KEY = "surt-prototype-choice";

export async function approveChoice(choice: Choice, notes: string) {
  const record = {
    ...choice,
    summary: describeChoice(choice),
    notes,
    approvedAt: new Date().toISOString(),
  };
  if (isTauri()) {
    const path = await invoke<string>("approve_choice", { choice: record });
    return `Gravado em ${path}`;
  }
  remember(JSON.stringify(record));
  try {
    await navigator.clipboard.writeText(JSON.stringify(record, null, 2));
    return "Salvo neste navegador e copiado para a área de transferência.";
  } catch {
    return "Salvo neste navegador.";
  }
}

function remember(serialized: string) {
  try {
    localStorage.setItem(STORAGE_KEY, serialized);
  } catch {
    return;
  }
}
