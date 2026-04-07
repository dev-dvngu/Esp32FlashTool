<script setup lang="ts">
import { useMainStore, type ChipProfile } from '../../stores/main';
import HexInput from '../../components/shared/HexInput.vue';
import { open } from '@tauri-apps/plugin-dialog';
import { startFlash, cancelFlash, eraseFlash, type FlashStartPayload } from '../../api/commands';
import { watch } from 'vue';
import { Plus, Trash2 } from 'lucide-vue-next';

const store = useMainStore();

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
  if (!store.port) { alert("Please select a port!"); return; }
  const enabledItems = store.flashItems.filter(i => i.enabled && i.filePath.trim() !== '');
  if (enabledItems.length === 0) { alert("Select at least one valid file!"); return; }
  
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
    items: enabledItems.map(i => ({ offset: i.offset, file_path: i.filePath })),
    extra_args: store.extraArgs
  };

  try {
    await startFlash(payload);
  } catch (err: any) {
    store.isBusy = false;
    store.status = 'error';
    store.addLog(`Flash failed: ${err}`, "error");
  }
};

const handleErase = async () => {
  if (!store.port) { alert("Please select a port!"); return; }
  if (!confirm("Erase entire flash?")) return;
  
  store.clearLogs();
  store.isBusy = true;
  store.status = 'connecting';
  store.progress = 0;
  try {
    await eraseFlash(store.port, parseInt(store.baudrate, 10));
  } catch (err: any) {
    store.isBusy = false;
    store.status = 'error';
    store.addLog(`Erase failed: ${err}`, "error");
  }
};

const handleCancel = async () => {
  try { await cancelFlash(); } catch (err: any) { store.addLog(`Cancel failed: ${err}`, "error"); }
};

const onProfileChange = (e: Event) => {
  const target = e.target as HTMLSelectElement;
  store.setProfile(target.value as ChipProfile);
};
</script>

<template>
  <div class="h-full flex flex-col bg-[#121212] text-[#e0e0e0]">
    
    <!-- Action Bar (MUI Surface) -->
    <div class="bg-[#1e1e1e] border-b border-[#333333] flex flex-col shrink-0 z-10 shadow-sm">
      
      <!-- Primary Config & Major Actions -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-[#333333]">
        <div class="flex items-center space-x-6">
          <div class="flex items-center space-x-3">
            <label class="text-xs font-bold text-[#9e9e9e] uppercase tracking-wider">Profile</label>
            <select :value="store.chipProfile" @change="onProfileChange" class="border border-[#444] rounded-md px-3 py-1.5 text-sm outline-none bg-[#121212] text-[#90caf9] font-bold cursor-pointer min-w-[120px] focus:border-[#90caf9] focus:ring-1 focus:ring-[#90caf9] transition-all">
              <option value="esp32" class="bg-[#1e1e1e] text-[#e0e0e0]">ESP32</option>
              <option value="esp8266" class="bg-[#1e1e1e] text-[#e0e0e0]">ESP8266</option>
              <option value="custom" class="bg-[#1e1e1e] text-[#e0e0e0]">Custom</option>
            </select>
          </div>

          <div class="h-6 w-px bg-[#333333]"></div>

          <div class="flex items-center space-x-3">
            <button v-if="store.isBusy" @click="handleCancel" class="bg-[#d32f2f] hover:bg-[#b71c1c] text-white px-6 py-1.5 text-sm rounded-md shadow-sm font-bold uppercase transition-all focus:outline-none focus:ring-2 focus:ring-[#f44336] focus:ring-offset-2 focus:ring-offset-[#1e1e1e]">
              Cancel Operation
            </button>
            <template v-else>
              <button @click="handleErase" class="bg-[#121212] hover:bg-[#2a2a2a] text-[#f44336] border border-[#f44336]/50 hover:border-[#f44336] px-4 py-1.5 text-sm rounded-md shadow-sm font-bold transition-all focus:outline-none focus:ring-2 focus:ring-[#f44336] focus:ring-offset-2 focus:ring-offset-[#1e1e1e]">
                Erase Chip
              </button>
              <button @click="handleWrite" class="bg-[#1976d2] hover:bg-[#1565c0] text-white px-8 py-1.5 text-sm rounded-md shadow-sm font-bold uppercase tracking-wider transition-all focus:outline-none focus:ring-2 focus:ring-[#90caf9] focus:ring-offset-2 focus:ring-offset-[#1e1e1e]">
                Program
              </button>
            </template>
          </div>
        </div>

        <button @click="store.resetDefaultFiles()" class="text-xs text-[#90caf9] hover:underline font-medium opacity-80 hover:opacity-100 transition-opacity focus:outline-none">
          Reset Defaults
        </button>
      </div>

      <!-- Advanced Options -->
      <div class="flex items-center justify-between px-4 py-2.5 bg-[#1a1a1a]">
        <div class="flex items-center space-x-8 text-xs font-semibold text-[#b3b3b3]">
          <div class="flex items-center space-x-5">
            <div class="flex items-center space-x-2">
              <label>Mode</label>
              <select v-model="store.flashMode" class="border border-[#444] rounded-md px-2 py-1 bg-[#121212] text-[#e0e0e0] font-normal cursor-pointer focus:border-[#90caf9] outline-none">
                <option value="dio" class="bg-[#1e1e1e] text-[#e0e0e0]">DIO</option>
                <option value="dout" class="bg-[#1e1e1e] text-[#e0e0e0]">DOUT</option>
                <option value="qio" class="bg-[#1e1e1e] text-[#e0e0e0]">QIO</option>
                <option value="qout" class="bg-[#1e1e1e] text-[#e0e0e0]">QOUT</option>
              </select>
            </div>
            
            <div class="flex items-center space-x-2">
              <label>Freq</label>
              <select v-model="store.flashFreq" class="border border-[#444] rounded-md px-2 py-1 bg-[#121212] text-[#e0e0e0] font-normal cursor-pointer focus:border-[#90caf9] outline-none">
                <option value="40m" class="bg-[#1e1e1e] text-[#e0e0e0]">40 MHz</option>
                <option value="80m" class="bg-[#1e1e1e] text-[#e0e0e0]">80 MHz</option>
                <option value="26m" class="bg-[#1e1e1e] text-[#e0e0e0]">26 MHz</option>
                <option value="20m" class="bg-[#1e1e1e] text-[#e0e0e0]">20 MHz</option>
              </select>
            </div>

            <div class="flex items-center space-x-2">
              <label>Size</label>
              <select v-model="store.flashSize" class="border border-[#444] rounded-md px-2 py-1 bg-[#121212] text-[#e0e0e0] font-normal cursor-pointer focus:border-[#90caf9] outline-none">
                <option value="keep" class="bg-[#1e1e1e] text-[#e0e0e0]">Keep</option>
                <option value="2MB" class="bg-[#1e1e1e] text-[#e0e0e0]">2 MB</option>
                <option value="4MB" class="bg-[#1e1e1e] text-[#e0e0e0]">4 MB</option>
                <option value="8MB" class="bg-[#1e1e1e] text-[#e0e0e0]">8 MB</option>
                <option value="16MB" class="bg-[#1e1e1e] text-[#e0e0e0]">16 MB</option>
              </select>
            </div>
          </div>

          <div class="h-4 w-px bg-[#333333]"></div>

          <div class="flex items-center space-x-6">
            <label class="flex items-center space-x-2 cursor-pointer hover:text-[#e0e0e0] transition-colors group">
              <div class="relative flex items-center justify-center">
                 <input type="checkbox" v-model="store.eraseBefore" class="appearance-none w-4 h-4 rounded border-2 border-[#555] checked:bg-[#90caf9] checked:border-[#90caf9] cursor-pointer transition-all peer">
                 <svg class="absolute w-3 h-3 text-[#121212] opacity-0 peer-checked:opacity-100 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
              </div>
              <span>Erase before</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- Firmware Files Table -->
    <div class="flex-1 bg-[#1e1e1e] overflow-hidden flex flex-col relative">
      <div class="overflow-y-auto custom-scrollbar flex-1">
        <table class="w-full text-left whitespace-nowrap">
          <thead class="bg-[#2a2a2a] text-xs uppercase text-[#9e9e9e] font-bold sticky top-0 z-10 shadow-sm border-b border-[#333333]">
            <tr>
              <th class="px-3 py-3 w-10 text-center">
                 <div class="w-4 h-4 border-2 border-[#555] rounded opacity-50 mx-auto bg-[#444]"></div>
              </th>
              <th class="px-3 py-3 border-l border-[#333333] w-48">Description</th>
              <th class="px-3 py-3 border-l border-[#333333] w-32">Address</th>
              <th class="px-3 py-3 border-l border-[#333333]">File Path</th>
              <th class="px-3 py-3 border-l border-[#333333] w-12 text-center"></th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[#2a2a2a] text-sm">
            <tr v-for="(item, idx) in store.flashItems" :key="item.id" class="hover:bg-[#252525] transition-colors" :class="idx % 2 === 0 ? 'bg-[#1e1e1e]' : 'bg-[#1a1a1a]'">
              <td class="px-3 py-2 text-center">
                <div class="relative flex items-center justify-center">
                   <input type="checkbox" v-model="item.enabled" class="appearance-none w-4 h-4 rounded border-2 border-[#555] checked:bg-[#90caf9] checked:border-[#90caf9] cursor-pointer transition-all peer">
                   <svg class="absolute w-3 h-3 text-[#121212] opacity-0 peer-checked:opacity-100 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                </div>
              </td>
              <td class="px-3 py-2 border-l border-[#333333] font-medium text-[#cccccc]">
                <input 
                  v-if="store.chipProfile === 'custom'" 
                  v-model="item.description" 
                  type="text" 
                  class="bg-transparent border-b border-dashed border-[#555] outline-none w-full focus:border-[#90caf9] py-0.5 text-[#e0e0e0]"
                />
                <span v-else>{{ item.description }}</span>
              </td>
              <td class="px-3 py-2 border-l border-[#333333]">
                <HexInput v-model="item.offset" class="w-full" />
              </td>
              <td class="px-3 py-2 border-l border-[#333333]">
                <div class="flex items-center space-x-2">
                  <input type="text" readonly :value="item.filePath" class="flex-1 bg-[#121212] border border-[#444] rounded-md px-2.5 py-1.5 text-xs text-[#e0e0e0] outline-none cursor-default placeholder-[#757575] focus:border-[#90caf9] transition-colors" placeholder="No file selected...">
                  <button @click="pickFile(item.id)" class="px-4 py-1.5 bg-[#333] hover:bg-[#444] text-[#e0e0e0] border border-[#555] rounded-md transition-colors shadow-sm font-bold text-xs uppercase tracking-wider outline-none focus:ring-2 focus:ring-[#90caf9]">
                    Browse
                  </button>
                </div>
              </td>
              <td class="px-3 py-2 border-l border-[#333333] text-center">
                <button v-if="store.chipProfile === 'custom'" @click="store.removeFlashItem(item.id)" class="text-[#757575] hover:text-[#f44336] transition-colors p-1 outline-none focus:text-[#f44336]" title="Delete segment">
                  <Trash2 class="w-4 h-4 mx-auto" />
                </button>
              </td>
            </tr>
            
            <tr v-if="store.chipProfile === 'custom'" class="bg-[#1e1e1e] border-t border-[#333333]">
              <td colspan="5" class="p-0">
                <button @click="store.addFlashItem()" class="w-full py-3 text-[#90caf9] hover:bg-[#232323] flex items-center justify-center space-x-2 text-xs font-bold transition-colors outline-none focus:bg-[#232323]">
                  <Plus class="w-4 h-4" />
                  <span>ADD NEW SEGMENT</span>
                </button>
              </td>
            </tr>

            <tr v-if="store.flashItems.length === 0">
              <td colspan="5" class="text-center py-10 text-[#757575] italic text-sm bg-[#121212]/50">
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
tbody tr {
  min-height: 48px;
}
</style>
