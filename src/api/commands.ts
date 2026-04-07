import { invoke } from "@tauri-apps/api/core";

export interface SerialPortInfo {
  name: string;
  port_type: string;
}

export interface FlashItem {
  offset: string;
  file_path: string;
}

export interface FlashStartPayload {
  port: string;
  baud: number;
  flash_mode?: string;
  flash_freq?: string;
  flash_size?: string;
  erase_before: boolean;
  items: FlashItem[];
  extra_args?: string;
}

export async function greet(name: string): Promise<string> {
  return await invoke<string>("greet", { name });
}

export async function getEsptoolVersion(): Promise<string> {
  return await invoke<string>("esptool_version");
}

export async function getChipInfo(port: string, baud: number): Promise<string> {
  return await invoke<string>("get_chip_info", { port, baud });
}

export async function listSerialPorts(): Promise<SerialPortInfo[]> {
  return await invoke<SerialPortInfo[]>("list_serial_ports");
}

export async function startFlash(payload: FlashStartPayload): Promise<void> {
  return await invoke<void>("flash_start", { payload });
}

export async function eraseFlash(port: string, baud: number): Promise<void> {
  return await invoke<void>("erase_flash", { payload: { port, baud } });
}

export async function cancelFlash(): Promise<void> {
  return await invoke<void>("flash_cancel");
}

export async function readLocalFile(path: string, offset: number, size: number): Promise<number[]> {
  return await invoke<number[]>("read_local_file", { path, offset, size });
}

export async function writeLocalFile(path: string, data: number[]): Promise<void> {
  return await invoke<void>("write_local_file", { path, data });
}

export async function readFlash(port: string, baud: number, offset: string, size: string, outputFile: string): Promise<void> {
  return await invoke<void>("read_flash", { payload: { port, baud, offset, size, output_file: outputFile } });
}
