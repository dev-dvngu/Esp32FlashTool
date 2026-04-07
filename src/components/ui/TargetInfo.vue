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
    
    // Regex siêu linh hoạt cho nhiều phiên bản esptool
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
  <div class="flex flex-col overflow-hidden shrink-0 shadow-sm border border-[#004bb0] rounded-sm flex-1 min-h-0">
    <!-- Header -->
    <div class="bg-[#003b8e] px-3 py-1.5 flex items-center justify-between border-b border-[#004bb0] shrink-0">
      <div class="flex items-center space-x-2">
        <Search class="w-3.5 h-3.5 text-[#a0d030]" />
        <h3 class="text-[10px] font-bold uppercase tracking-widest text-white">Target information</h3>
      </div>
      <button @click="getInfo" :disabled="store.isBusy || !store.port" class="text-white hover:text-[#a0d030] transition-colors p-0.5">
        <RefreshCw class="w-3.5 h-3.5" :class="{'animate-spin': store.isBusy && store.status === 'connecting'}" />
      </button>
    </div>
    
    <!-- Info Content (Optimized Row Layout) -->
    <div class="bg-[#001840] p-3 overflow-y-auto custom-scrollbar flex-1">
      <div v-if="store.chipInfo" class="space-y-1.5 font-mono text-[10px]">
        
        <!-- Standard fields in rows -->
        <div v-for="field in infoFields" :key="field.key" class="flex justify-between items-center border-b border-[#003b8e]/30 pb-1 last:border-0">
          <span class="text-[#a0d030] uppercase font-bold tracking-tighter opacity-80 shrink-0 mr-4">{{ field.label }}</span>
          <span class="text-blue-100 font-bold truncate">{{ (store.chipInfo as any)[field.key] }}</span>
        </div>

        <!-- Features field (Handles long text with wrap) -->
        <div class="flex flex-col pt-1">
          <span class="text-[#a0d030] uppercase font-bold tracking-tighter opacity-80 mb-1">Features</span>
          <span class="text-blue-100 leading-tight text-[9px] block bg-[#002a60]/50 p-1.5 rounded-sm border border-[#003b8e]/30">
            {{ store.chipInfo.features }}
          </span>
        </div>

      </div>
      
      <div v-else class="h-full flex flex-col items-center justify-center text-slate-500 italic text-[9px] opacity-20 py-10">
        <Search class="w-8 h-8 mb-2" />
        <span>Click refresh to scan device</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: #001840; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #003b8e; border-radius: 2px; }
</style>
