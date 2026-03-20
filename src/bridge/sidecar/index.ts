import { Command } from "@tauri-apps/plugin-shell";

export async function runSidecar(sidecarName: string, args: string[] = []) {
  const command = Command.sidecar(sidecarName, args);
  return await command.execute();
}

