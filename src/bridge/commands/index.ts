import { invoke } from "@tauri-apps/api/core";

export async function greet(name: string): Promise<string> {
  return await invoke<string>("greet", { name });
}

export async function esptoolVersion(): Promise<string> {
  return await invoke<string>("esptool_version");
}

export type SerialPortInfo = {
  name: string;
  port_type: string;
};

export async function listSerialPorts(): Promise<SerialPortInfo[]> {
  return await invoke<SerialPortInfo[]>("list_serial_ports");
}

export type FlashItem = {
  offset: string;
  file_path: string;
};

export type FlashStartPayload = {
  port: string;
  baud: number;
  flash_mode?: string;
  flash_freq?: string;
  flash_size?: string;
  erase_before: boolean;
  verify_after: boolean;
  items: FlashItem[];
  extra_args?: string;
};

export async function flashStart(payload: FlashStartPayload): Promise<void> {
  await invoke("flash_start", { payload });
}

export async function flashCancel(): Promise<void> {
  await invoke("flash_cancel");
}

