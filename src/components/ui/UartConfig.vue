<script setup lang="ts">
import { useMainStore } from '../../stores/main';
import { listSerialPorts } from '../../api/commands';
import { RefreshCw, Settings2 } from 'lucide-vue-next';

const store = useMainStore();
const baudrates = ['9600', '115200', '230400', '460800', '921600'];

const scanPorts = async () => {
  store.isScanning = true;
  try {
    const ports = await listSerialPorts();
    store.ports = ports.map(p => p.name);
    if (store.ports.length > 0 && !store.port) store.port = store.ports[0];
  } catch (e) {
    store.addLog(`Port scan failed: ${e}`, 'error');
  } finally {
    store.isScanning = false;
  }
};
</script>

<template>
  <div class="flex flex-col overflow-hidden shrink-0 shadow-sm border border-[#004bb0] rounded-sm">
    <div class="bg-[#003b8e] px-3 py-1.5 flex items-center space-x-2 border-b border-[#004bb0]">
      <Settings2 class="w-3.5 h-3.5 text-[#a0d030]" />
      <h3 class="text-[10px] font-bold uppercase tracking-widest text-white">UART configuration</h3>
    </div>
    <div class="bg-[#001840] p-3 space-y-3">
      <div class="flex items-center justify-between">
        <label class="text-[9px] text-[#a0d030] font-bold uppercase">Port</label>
        <div class="flex w-36 bg-white rounded-sm overflow-hidden border border-slate-400 shadow-inner">
          <select v-model="store.port" :disabled="store.isBusy" class="flex-1 text-slate-900 text-[10px] px-1 py-0.5 outline-none border-none font-bold">
            <option v-for="p in store.ports" :key="p" :value="p">{{ p }}</option>
          </select>
          <button @click="scanPorts" :disabled="store.isScanning || store.isBusy" class="px-1 bg-slate-100 text-slate-600 hover:bg-slate-200 border-l border-slate-300">
            <RefreshCw class="w-2.5 h-2.5" :class="{'animate-spin': store.isScanning}" />
          </button>
        </div>
      </div>
      <div class="flex items-center justify-between">
        <label class="text-[9px] text-[#a0d030] font-bold uppercase">Baudrate</label>
        <select v-model="store.baudrate" :disabled="store.isBusy" class="w-36 text-slate-900 bg-white text-[10px] px-1 py-0.5 rounded-sm outline-none border border-slate-400 font-bold">
          <option v-for="b in baudrates" :key="b" :value="b">{{ b }}</option>
        </select>
      </div>
    </div>
  </div>
</template>
