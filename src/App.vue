<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { FlashStartPayload } from "./bridge/commands";
import {
  esptoolVersion,
  flashCancel,
  flashStart,
  listSerialPorts,
} from "./bridge/commands";

type FlashRow = { name: string; offset: string; filePath: string; placeholder: string };

const ready = ref(false);
const readyMsg = ref<string>("");

const ports = ref<string[]>([]);
const port = ref<string>("");
const baud = ref<number>(115200);

const rows = ref<FlashRow[]>([
  { name: "Bootloader", offset: "0x1000", filePath: "", placeholder: "example.bootloader.bin" },
  { name: "Partition Table", offset: "0x8000", filePath: "", placeholder: "example.partitions.bin" },
  { name: "boot_app0", offset: "0xe000", filePath: "", placeholder: "boot_app0.bin" },
  { name: "Application", offset: "0x10000", filePath: "", placeholder: "example.bin" },
]);
const eraseBefore = ref(false);
const verifyAfter = ref(true);

const progress = ref<number>(0);
const status = ref<string>("idle");
const logs = ref<string[]>([]);
const running = computed(() =>
  status.value === "connecting" || status.value === "writing" || status.value === "erasing"
);

async function refreshPorts() {
  const list = await listSerialPorts();
  ports.value = list.map((p) => p.name);
  if (!port.value && ports.value.length) port.value = ports.value[0];
}

async function pickFile(i: number) {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Firmware", extensions: ["bin"] }],
  });
  if (typeof selected === "string") {
    rows.value[i].filePath = selected;
  }
}

function addRow() {
  if (rows.value.length >= 8) return;
  rows.value.push({ name: "", offset: "0x0", filePath: "", placeholder: "example.bin" });
}

function removeRow(i: number) {
  if (rows.value.length <= 1) return;
  rows.value.splice(i, 1);
}

async function startFlash() {
  logs.value = [];
  progress.value = 0;
  status.value = "connecting";

  const items = rows.value
    .filter((r) => r.filePath.trim().length > 0)
    .map((r) => ({ offset: r.offset.trim(), file_path: r.filePath }));

  if (items.length === 0) {
    logs.value.push("[ERROR] Chưa chọn file .bin nào!");
    status.value = "error";
    return;
  }

  const payload: FlashStartPayload = {
    port: port.value,
    baud: baud.value,
    erase_before: eraseBefore.value,
    verify_after: verifyAfter.value,
    items,
  };

  try {
    await flashStart(payload);
  } catch (e) {
    logs.value.push(`[ERROR] ${String(e)}`);
    status.value = "error";
  }
}

async function cancelFlash() {
  await flashCancel();
}

onMounted(async () => {
  try {
    const v = await esptoolVersion();
    ready.value = true;
    readyMsg.value = v ? `esptool: ${v}` : "esptool: OK";
  } catch (e) {
    ready.value = false;
    readyMsg.value = `esptool lỗi: ${String(e)}`;
  }

  await refreshPorts();

  await listen<{ text: string; level: string }>("log_line", (ev) => {
    logs.value.push(ev.payload.text);
  });
  await listen<{ percent: number }>("progress", (ev) => {
    progress.value = ev.payload.percent;
  });
  await listen<{ status: string }>("status_change", (ev) => {
    status.value = ev.payload.status;
  });
  await listen<{ success: boolean }>("completed", (ev) => {
    status.value = ev.payload.success ? "done" : "error";
  });
});
</script>

<template>
  <main class="page">
    <header class="header">
      <div>
        <div class="title">ESP32 Flash Tool (Pre-loading)</div>
        <div class="subtitle" :class="ready ? 'ok' : 'bad'">{{ readyMsg }}</div>
      </div>
      <div class="status">
        <span class="pill" :class="status">{{ status }}</span>
        <span class="pill">{{ progress }}%</span>
      </div>
    </header>

    <section class="card">
      <div class="grid">
        <label>
          <div class="label">Serial Port</div>
          <div class="row">
            <select v-model="port" :disabled="running">
              <option v-for="p in ports" :key="p" :value="p">{{ p }}</option>
            </select>
            <button class="ghost" type="button" @click="refreshPorts" :disabled="running">
              Refresh
            </button>
          </div>
        </label>

        <label>
          <div class="label">Baud</div>
          <select v-model.number="baud" :disabled="running">
            <option :value="115200">115200</option>
            <option :value="460800">460800</option>
            <option :value="921600">921600</option>
          </select>
        </label>

        <label class="check">
          <input type="checkbox" v-model="eraseBefore" :disabled="running" />
          <span>Erase trước khi flash</span>
        </label>

        <label class="check">
          <input type="checkbox" v-model="verifyAfter" :disabled="running" />
          <span>Verify sau khi flash</span>
        </label>
      </div>
    </section>

    <section class="card">
      <div class="cardHeader">
        <div class="cardTitle">Files & Offsets</div>
        <button class="ghost" type="button" @click="addRow" :disabled="running || rows.length >= 8">
          + Add row
        </button>
      </div>

      <div class="table">
        <div class="thead">
          <div>Name</div>
          <div>Offset</div>
          <div>File</div>
          <div></div>
        </div>
        <div v-for="(r, i) in rows" :key="i" class="trow">
          <input v-model="r.name" :disabled="running" placeholder="Label..." />
          <input v-model="r.offset" :disabled="running" />
          <div class="row">
            <input v-model="r.filePath" :disabled="running" :placeholder="r.placeholder || 'example.bin'" />
            <button class="ghost" type="button" @click="pickFile(i)" :disabled="running">
              Browse
            </button>
          </div>
          <button class="ghost" type="button" @click="removeRow(i)" :disabled="running || rows.length <= 1">
            Remove
          </button>
        </div>
      </div>

      <div class="actions">
        <button class="primary" type="button" @click="startFlash" :disabled="!ready || running || !port">
          Flash
        </button>
        <button class="danger" type="button" @click="cancelFlash" :disabled="!running">
          Cancel
        </button>
      </div>
    </section>

    <section class="card">
      <div class="cardHeader">
        <div class="cardTitle">Log</div>
      </div>
      <pre class="log">{{ logs.join("\n") }}</pre>
    </section>
  </main>
</template>

<style>
:root {
  font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Inter, Arial, sans-serif;
  font-size: 14px;
  line-height: 20px;
  color: #e6edf3;
  background-color: #0b1220;
}

body {
  margin: 0;
}

.page {
  padding: 16px;
  max-width: 1100px;
  margin: 0 auto;
  display: grid;
  gap: 12px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: end;
  gap: 12px;
}

.title {
  font-size: 16px;
  font-weight: 700;
}

.subtitle {
  font-size: 12px;
  opacity: 0.9;
}
.subtitle.ok { color: #7ee787; }
.subtitle.bad { color: #ff7b72; }

.status {
  display: flex;
  gap: 8px;
}

.pill {
  border: 1px solid rgba(255,255,255,0.12);
  background: rgba(255,255,255,0.06);
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
}

.pill.done { border-color: rgba(126,231,135,0.35); }
.pill.error { border-color: rgba(255,123,114,0.35); }
.pill.writing, .pill.erasing, .pill.connecting { border-color: rgba(121,192,255,0.35); }

.card {
  border: 1px solid rgba(255,255,255,0.12);
  background: rgba(255,255,255,0.04);
  border-radius: 12px;
  padding: 12px;
}

.cardHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.cardTitle {
  font-weight: 700;
}

.grid {
  display: grid;
  grid-template-columns: 1.5fr 1fr 1fr 1fr;
  gap: 10px;
  align-items: end;
}

.label {
  font-size: 12px;
  opacity: 0.85;
  margin-bottom: 4px;
}

.row {
  display: flex;
  gap: 8px;
  align-items: center;
}

input, select, button {
  height: 34px;
  border-radius: 10px;
  border: 1px solid rgba(255,255,255,0.14);
  background: rgba(255,255,255,0.06);
  color: #e6edf3;
  padding: 0 10px;
}

select {
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23999' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  padding-right: 28px;
}

select option {
  background: #161b22;
  color: #e6edf3;
}

input:disabled, select:disabled, button:disabled {
  opacity: 0.55;
}

button {
  cursor: pointer;
}

button.ghost {
  background: transparent;
}

button.primary {
  background: #1f6feb;
  border-color: rgba(31,111,235,0.7);
}

button.danger {
  background: rgba(255,123,114,0.18);
  border-color: rgba(255,123,114,0.35);
}

.check {
  display: flex;
  gap: 8px;
  align-items: center;
  user-select: none;
}
.check input {
  width: 16px;
  height: 16px;
  border-radius: 4px;
}

.table {
  display: grid;
  gap: 8px;
}
.thead, .trow {
  display: grid;
  grid-template-columns: 140px 100px 1fr 80px;
  gap: 8px;
  align-items: center;
}
.thead {
  font-size: 12px;
  opacity: 0.85;
}

.actions {
  margin-top: 10px;
  display: flex;
  gap: 10px;
}

.log {
  margin: 0;
  padding: 10px;
  height: 260px;
  overflow: auto;
  background: #0d1117;
  border-radius: 10px;
  border: 1px solid rgba(255,255,255,0.12);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
}
</style>
