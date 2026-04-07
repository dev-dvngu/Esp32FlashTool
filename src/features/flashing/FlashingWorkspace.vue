<script setup lang="ts">
import { useMainStore, type ChipProfile } from '../../stores/main';
import HexInput from '../../components/shared/HexInput.vue';
import { open } from '@tauri-apps/plugin-dialog';
import { startFlash, cancelFlash, eraseFlash, type FlashStartPayload } from '../../api/commands';
import { watch } from 'vue';
import { Plus, Trash2 } from 'lucide-vue-next';

const store = useMainStore();

// Tự động lưu cấu hình khi có thay đổi
watch(() => store.$state, () => {
  store.saveSettings();
}, { deep: true });

const pickFile = async (id: string) => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Binary', extensions: ['bin'] }]
  });
  if (selected && typeof selected === 'string') {
    const item = store.flashItems.find(i => i.id === id);
    if (item) item.filePath = selected;
  }
};

const handleWrite = async () => {
  if (!store.port) {
    alert("Please select a serial port first!");
    return;
  }
  const enabledItems = store.flashItems.filter(i => i.enabled && i.filePath.trim() !== '');
  if (enabledItems.length === 0) {
    alert("Please select at least one valid binary file to flash!");
    return;
  }
  store.clearLogs();
  store.isBusy = true;
  store.status = 'connecting';
  store.progress = 0;

  const payload: FlashStartPayload = {
    port: store.port,
    baud: parseInt(store.baudrate, 10),
    flash_mode: store.flashMode,
    flash_freq: store.flashFreq,
    flash_size: store.flashSize !== 'keep' ? store.flashSize : undefined,
    erase_before: store.eraseBefore,
    verify_after: store.verifyAfter,
    items: enabledItems.map(i => ({ offset: i.offset, file_path: i.filePath })),
    extra_args: store.extraArgs
  };

  try {
    await startFlash(payload);
  } catch (err: any) {
    store.isBusy = false;
    store.status = 'error';
    store.addLog(`Flash start failed: ${err}`, "error");
  }
};

const handleErase = async () => {
  if (!store.port) {
    alert("Please select a serial port first!");
    return;
  }
  if (!confirm("WARNING: This will erase the entire flash. Continue?")) return;
  
  store.clearLogs();
  store.isBusy = true;
  store.status = 'connecting';
  store.progress = 0;
  try {
    await eraseFlash(store.port, parseInt(store.baudrate, 10));
  } catch (err: any) {
    store.isBusy = false;
    store.status = 'error';
    store.addLog(`Erase start failed: ${err}`, "error");
  }
};

const handleCancel = async () => {
  try {
    await cancelFlash();
  } catch (err: any) {
    store.addLog(`Cancel failed: ${err}`, "error");
  }
};

const onProfileChange = (e: Event) => {
  const target = e.target as HTMLSelectElement;
  store.setProfile(target.value as ChipProfile);
};
</script>

<template>
  <div class="h-full flex flex-col bg-white">
    
    <!-- Action Bar (Grouped into 2 lines for readability) -->
    <div class="bg-[#f0f2f5] border-b border-slate-300 shadow-sm flex flex-col shrink-0 z-10">
      
      <!-- Line 1: Primary Config & Major Actions -->
      <div class="flex items-center justify-between px-4 py-2 border-b border-slate-200">
        <div class="flex items-center space-x-6">
          <!-- Profile -->
          <div class="flex items-center space-x-3">
            <label class="text-xs font-bold text-[#002052] uppercase tracking-wider">Chip Profile</label>
            <select :value="store.chipProfile" @change="onProfileChange" class="border border-[#2b90d9] rounded-sm px-3 py-1 text-sm outline-none bg-blue-50 font-bold text-[#2b90d9] cursor-pointer min-w-[120px]">
              <option value="esp32">ESP32</option>
              <option value="esp8266">ESP8266</option>
              <option value="custom">Custom</option>
            </select>
          </div>

          <div class="h-6 w-px bg-slate-300"></div>

          <!-- Major Actions -->
          <div class="flex items-center space-x-3">
            <button v-if="store.isBusy" @click="handleCancel" class="bg-[#e5004f] hover:bg-[#c00040] text-white px-6 py-1.5 text-sm rounded-sm shadow-sm font-bold uppercase transition-all">
              Cancel Operation
            </button>
            <template v-else>
              <button @click="handleErase" class="bg-white hover:bg-slate-100 text-slate-700 border border-slate-300 px-4 py-1.5 text-sm rounded-sm shadow-sm font-bold transition-all">
                Full Chip Erase
              </button>
              <button @click="handleWrite" class="bg-[#2b90d9] hover:bg-[#207bbf] text-white px-8 py-1.5 text-sm rounded-sm shadow-sm font-bold uppercase tracking-wider transition-all">
                Program
              </button>
            </template>
          </div>
        </div>

        <button @click="store.resetDefaultFiles()" class="text-xs text-[#2b90d9] hover:underline font-semibold">
          Reset to Defaults
        </button>
      </div>

      <!-- Line 2: Advanced Options -->
      <div class="flex items-center justify-between px-4 py-2 bg-slate-50">
        <div class="flex items-center space-x-8 text-xs font-semibold text-slate-600">
          <!-- Mode/Freq/Size Group -->
          <div class="flex items-center space-x-4">
            <div class="flex items-center space-x-2">
              <label>Flash Mode</label>
              <select v-model="store.flashMode" class="border border-slate-300 rounded-sm px-2 py-0.5 bg-white text-slate-900 font-normal cursor-pointer">
                <option value="dio">DIO</option>
                <option value="dout">DOUT</option>
                <option value="qio">QIO</option>
                <option value="qout">QOUT</option>
              </select>
            </div>
            
            <div class="flex items-center space-x-2">
              <label>Frequency</label>
              <select v-model="store.flashFreq" class="border border-slate-300 rounded-sm px-2 py-0.5 bg-white text-slate-900 font-normal cursor-pointer">
                <option value="40m">40 MHz</option>
                <option value="80m">80 MHz</option>
                <option value="26m">26 MHz</option>
                <option value="20m">20 MHz</option>
              </select>
            </div>

            <div class="flex items-center space-x-2">
              <label>Flash Size</label>
              <select v-model="store.flashSize" class="border border-slate-300 rounded-sm px-2 py-0.5 bg-white text-slate-900 font-normal cursor-pointer">
                <option value="keep">Keep</option>
                <option value="2MB">2 MB</option>
                <option value="4MB">4 MB</option>
                <option value="8MB">8 MB</option>
                <option value="16MB">16 MB</option>
              </select>
            </div>
          </div>

          <div class="h-4 w-px bg-slate-200"></div>

          <!-- Checkboxes -->
          <div class="flex items-center space-x-6">
            <label class="flex items-center space-x-2 cursor-pointer hover:text-slate-900 transition-colors">
              <input type="checkbox" v-model="store.eraseBefore" class="w-4 h-4 text-[#2b90d9] border-slate-300 rounded-sm focus:ring-[#2b90d9]">
              <span>Erase sectors before programming</span>
            </label>
            <label class="flex items-center space-x-2 cursor-pointer hover:text-slate-900 transition-colors">
              <input type="checkbox" v-model="store.verifyAfter" class="w-4 h-4 text-[#2b90d9] border-slate-300 rounded-sm focus:ring-[#2b90d9]">
              <span>Verify programming</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- Firmware Files Table -->
    <div class="flex-1 bg-white overflow-hidden flex flex-col relative">
      <div class="overflow-y-auto custom-scrollbar flex-1">
        <table class="w-full text-left whitespace-nowrap">
          <thead class="bg-slate-100 text-xs uppercase text-slate-500 font-bold sticky top-0 z-10 shadow-sm border-b border-slate-300">
            <tr>
              <th class="px-3 py-2 w-10 text-center">En</th>
              <th class="px-3 py-2 border-l border-slate-200 w-48">Description</th>
              <th class="px-3 py-2 border-l border-slate-200 w-32">Address</th>
              <th class="px-3 py-2 border-l border-slate-200">File Path</th>
              <th class="px-3 py-2 border-l border-slate-200 w-12 text-center"></th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100 text-sm">
            <tr v-for="(item, idx) in store.flashItems" :key="item.id" class="hover:bg-blue-50/50 transition-colors" :class="idx % 2 === 0 ? 'bg-white' : 'bg-[#fcfdfe]'">
              <td class="px-3 py-2 text-center">
                <input type="checkbox" v-model="item.enabled" class="cursor-pointer w-4 h-4 text-[#2b90d9] border-slate-300 rounded-sm focus:ring-[#2b90d9]">
              </td>
              <td class="px-3 py-2 border-l border-slate-100 font-medium text-slate-700">
                <input 
                  v-if="store.chipProfile === 'custom'" 
                  v-model="item.description" 
                  type="text" 
                  class="bg-transparent border-b border-dashed border-slate-300 outline-none w-full focus:border-blue-400 py-0.5 italic"
                />
                <span v-else>{{ item.description }}</span>
              </td>
              <td class="px-3 py-2 border-l border-slate-100">
                <HexInput v-model="item.offset" class="w-full" />
              </td>
              <td class="px-3 py-2 border-l border-slate-100">
                <div class="flex items-center space-x-2">
                  <input type="text" readonly :value="item.filePath" class="flex-1 bg-slate-50 border border-slate-200 rounded-sm px-2 py-1 text-xs text-slate-600 outline-none cursor-default" placeholder="No file selected...">
                  <button @click="pickFile(item.id)" class="px-3 py-1 bg-[#2b90d9] hover:bg-[#207bbf] text-white rounded-sm transition-colors shadow-sm font-bold text-xs uppercase tracking-tighter">
                    Browse
                  </button>
                </div>
              </td>
              <td class="px-3 py-2 border-l border-slate-100 text-center">
                <button v-if="store.chipProfile === 'custom'" @click="store.removeFlashItem(item.id)" class="text-slate-400 hover:text-red-600 transition-colors p-1" title="Delete segment">
                  <Trash2 class="w-4 h-4 mx-auto" />
                </button>
              </td>
            </tr>
            
            <!-- Add New Segment Row (Only for Custom) -->
            <tr v-if="store.chipProfile === 'custom'" class="bg-white border-t border-slate-200">
              <td colspan="5" class="p-0">
                <button @click="store.addFlashItem()" class="w-full py-2 text-blue-600 hover:bg-blue-50 flex items-center justify-center space-x-2 text-xs font-bold transition-colors">
                  <Plus class="w-4 h-4" />
                  <span>ADD NEW SEGMENT</span>
                </button>
              </td>
            </tr>

            <tr v-if="store.flashItems.length === 0">
              <td colspan="5" class="text-center py-10 text-slate-400 italic text-sm bg-slate-50/20">
                No flash segments defined. Click "Add" to create one.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

  </div>
</template>

<style scoped>
/* Ensure table rows have a minimum height for better target size */
tbody tr {
  min-height: 40px;
}

/* Custom scrollbar for the table area */
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 3px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #94a3b8;
}
</style>
