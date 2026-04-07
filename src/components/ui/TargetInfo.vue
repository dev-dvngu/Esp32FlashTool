<script setup lang="ts">
import { useMainStore } from '../../stores/main';
import { getChipInfo } from '../../api/commands';
import { RefreshCw, Search } from 'lucide-vue-next';

const store = useMainStore();

const getInfo = async () => {
  if (!store.port) return;
  store.isBusy = true;
  store.status = 'connecting';
  try {
    const rawInfo = await getChipInfo(store.port, parseInt(store.baudrate, 10));
    const chipTypeMatch = rawInfo.match(/(?:Chip type|Chip is):\s*([^\n\(]+)/i);
    const revMatch = rawInfo.match(/revision\s*([^\s\n,)]+)/i);
    const featuresMatch = rawInfo.match(/Features:\s*([^\n]+)/i);
    const crystalMatch = rawInfo.match(/(?:Crystal frequency|Crystal is):\s*([^\n]+)/i);
    const macMatch = rawInfo.match(/MAC:\s*([a-f0-9:]{17})/i);
    
    store.chipInfo = {
      chipType: chipTypeMatch ? chipTypeMatch[1].trim() : 'ESP32',
      revision: revMatch ? revMatch[1].trim() : 'N/A',
      mac: macMatch ? macMatch[1].toUpperCase() : 'N/A',
      crystal: crystalMatch ? crystalMatch[1].trim() : 'N/A',
      features: featuresMatch ? featuresMatch[1].trim() : 'N/A'
    };
    store.addLog("Chip information updated.", "success");
  } catch (e) {
    store.addLog(`Failed to get chip info: ${e}`, 'error');
  } finally {
    store.isBusy = false;
    store.status = 'idle';
  }
};

const infoFields = [
  { key: 'chipType', label: 'Chip Type' },
  { key: 'revision', label: 'Revision' },
  { key: 'mac', label: 'MAC Address' },
  { key: 'crystal', label: 'Crystal' },
];
</script>

<template>
  <div class="flex flex-col overflow-hidden shrink-0 shadow-md border border-[#333333] rounded-lg bg-[#1e1e1e] flex-1 min-h-0">
    <div class="bg-[#232323] px-4 py-3 flex items-center justify-between border-b border-[#333333] shrink-0">
      <div class="flex items-center space-x-2">
        <Search class="w-4 h-4 text-[#90caf9]" />
        <h3 class="text-xs font-bold uppercase tracking-wider text-[#e0e0e0]">Target Info</h3>
      </div>
      <button @click="getInfo" :disabled="store.isBusy || !store.port" class="text-[#9e9e9e] hover:text-[#90caf9] transition-colors p-1 rounded-full hover:bg-[#333] disabled:opacity-50 outline-none focus:ring-2 focus:ring-[#90caf9]">
        <RefreshCw class="w-3.5 h-3.5" :class="{'animate-spin': store.isBusy && store.status === 'connecting'}" />
      </button>
    </div>
    <div class="p-4 overflow-y-auto custom-scrollbar flex-1 bg-[#1e1e1e]">
      <div v-if="store.chipInfo" class="space-y-3 font-mono text-[11px]">
        <div v-for="field in infoFields" :key="field.key" class="flex justify-between items-center border-b border-[#333333] pb-2 last:border-0">
          <span class="text-[#9e9e9e] uppercase font-bold tracking-tighter shrink-0 mr-4">{{ field.label }}</span>
          <span class="text-[#e0e0e0] font-medium truncate">{{ (store.chipInfo as any)[field.key] }}</span>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-[#9e9e9e] uppercase font-bold tracking-tighter mb-1.5">Features</span>
          <span class="text-[#b3b3b3] leading-relaxed text-[10px] block bg-[#121212] p-2.5 rounded-md border border-[#333333]">
            {{ store.chipInfo.features }}
          </span>
        </div>
      </div>
      <div v-else class="h-full flex flex-col items-center justify-center text-[#757575] italic text-[10px] opacity-60 py-8">
        <Search class="w-8 h-8 mb-3 opacity-50" />
        <span>Click refresh to scan device</span>
      </div>
    </div>
  </div>
</template>
