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
  <div class="flex flex-col overflow-hidden shrink-0 shadow-md border border-[#333333] rounded-lg bg-[#1e1e1e]">
    <div class="px-4 py-3 flex items-center space-x-2 border-b border-[#333333] bg-[#232323]">
      <Settings2 class="w-4 h-4 text-[#90caf9]" />
      <h3 class="text-xs font-bold uppercase tracking-wider text-[#e0e0e0]">UART Config</h3>
    </div>
    
    <div class="p-4 space-y-4">
      <div class="flex flex-col space-y-1.5">
        <label class="text-[10px] text-[#9e9e9e] font-bold uppercase tracking-wide">Port</label>
        <div class="flex w-full bg-[#121212] rounded-md overflow-hidden border border-[#444] hover:border-[#90caf9] transition-colors focus-within:border-[#90caf9] focus-within:ring-1 focus-within:ring-[#90caf9]">
          <select v-model="store.port" :disabled="store.isBusy" class="flex-1 text-[#e0e0e0] text-xs px-3 py-2 outline-none border-none font-medium bg-[#121212] cursor-pointer disabled:opacity-50">
            <option value="" disabled class="bg-[#1e1e1e] text-[#e0e0e0]">Select Port</option>
            <option v-for="p in store.ports" :key="p" :value="p" class="bg-[#1e1e1e] text-[#e0e0e0]">{{ p }}</option>
          </select>
          <button @click="scanPorts" :disabled="store.isScanning || store.isBusy" class="px-3 bg-[#2a2a2a] text-[#b3b3b3] hover:text-[#90caf9] hover:bg-[#333] transition-colors border-l border-[#444] disabled:opacity-50 flex items-center justify-center outline-none">
            <RefreshCw class="w-3.5 h-3.5" :class="{'animate-spin': store.isScanning}" />
          </button>
        </div>
      </div>
      
      <div class="flex flex-col space-y-1.5">
        <label class="text-[10px] text-[#9e9e9e] font-bold uppercase tracking-wide">Baudrate</label>
        <div class="flex w-full bg-[#121212] rounded-md overflow-hidden border border-[#444] hover:border-[#90caf9] transition-colors focus-within:border-[#90caf9] focus-within:ring-1 focus-within:ring-[#90caf9]">
          <select v-model="store.baudrate" :disabled="store.isBusy" class="w-full text-[#e0e0e0] bg-[#121212] text-xs px-3 py-2 outline-none border-none font-medium cursor-pointer disabled:opacity-50">
            <option v-for="b in baudrates" :key="b" :value="b" class="bg-[#1e1e1e] text-[#e0e0e0]">{{ b }}</option>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>
