import { invoke } from "@tauri-apps/api/core";

export async function greet(name: string): Promise<string> {
  return await invoke<string>("greet", { name });
}

export async function esptoolVersion(): Promise<string> {
  return await invoke<string>("esptool_version");
}
