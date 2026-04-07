<script setup lang="ts">
import { ref, computed } from 'vue';
import { useMainStore } from '../../stores/main';
import { open, save } from '@tauri-apps/plugin-dialog';
import { Download, Plus, ChevronDown } from 'lucide-vue-next';
import { readLocalFile, writeLocalFile, readFlash, startFlash, type FlashStartPayload } from '../../api/commands';
import { listen } from '@tauri-apps/api/event';
import { tempDir, join } from '@tauri-apps/api/path';

const store = useMainStore();

interface MemoryTab {
  id: string;
  name: string;
  type: 'device' | 'file';
  path?: string;
  data: number[];
}

const tabs = ref<MemoryTab[]>([
  { id: 'device', name: 'Device Memory', type: 'device', data: [] },
]);

const activeTabId = ref('device');

const address = ref('0x0');
const size = ref('0x400');
const dataWidth = ref(32);
const isDropdownOpen = ref(false);

const MAX_DISPLAY_SIZE = 0x4000; 

const dataRows = computed(() => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (!currentTab || !currentTab.data.length) return [];
  
  const rows = [];
  const bytesPerRow = 16;
  
  const displayData = currentTab.data.length > MAX_DISPLAY_SIZE 
    ? currentTab.data.slice(0, MAX_DISPLAY_SIZE) 
    : currentTab.data;

  for (let i = 0; i < displayData.length; i += bytesPerRow) {
    const chunk = displayData.slice(i, i + bytesPerRow);
    const formattedWords = [];
    if (dataWidth.value === 32) {
      for (let j = 0; j < chunk.length; j += 4) {
        const val = (chunk[j] | (chunk[j+1]<<8) | (chunk[j+2]<<16) | (chunk[j+3]<<24)) >>> 0;
        formattedWords.push(val.toString(16).padStart(8, '0').toUpperCase());
      }
    } else if (dataWidth.value === 16) {
      for (let j = 0; j < chunk.length; j += 2) {
        const val = (chunk[j] | (chunk[j+1]<<8)) >>> 0;
        formattedWords.push(val.toString(16).padStart(4, '0').toUpperCase());
      }
    } else {
      for (let j = 0; j < chunk.length; j++) {
        formattedWords.push(chunk[j].toString(16).padStart(2, '0').toUpperCase());
      }
    }
    const ascii = chunk.map(b => (b >= 32 && b <= 126) ? String.fromCharCode(b) : '.').join('');
    rows.push({
      addr: '0x' + (parseInt(address.value, 16) + i).toString(16).padStart(8, '0').toUpperCase(),
      words: formattedWords,
      ascii
    });
  }
  return rows;
});

const closeTab = (id: string) => {
  if (id === 'device') return;
  const index = tabs.value.findIndex(t => t.id === id);
  if (index !== -1) {
    tabs.value.splice(index, 1);
    if (activeTabId.value === id) activeTabId.value = 'device';
  }
};

const pickFile = async () => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Binary', extensions: ['bin'] }]
  });
  if (selected && typeof selected === 'string') {
    const fileName = selected.split(/[\\/]/).pop() || 'Unknown.bin';
    const id = Math.random().toString(36).substr(2, 9);
    tabs.value.push({ id, name: fileName, type: 'file', path: selected, data: [] });
    activeTabId.value = id;
    await readFileData();
  }
};

const readFileData = async (readAll: boolean = false) => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (!currentTab) return;

  if (currentTab.type === 'device') {
    if (!store.port) { alert("Select Port!"); return; }
    try {
      const tDir = await tempDir();
      const tempFile = await join(tDir, 'esp_read_temp.bin');
      store.clearLogs(); store.isBusy = true; store.status = 'connecting'; store.progress = 0;

      let finalAddress = address.value.trim();
      if (!finalAddress || finalAddress === '0x') finalAddress = '0x0';
      let finalSize = size.value.trim();
      if (!finalSize || finalSize === '0x') finalSize = '0x400';
      if (readAll) { finalAddress = '0x0'; finalSize = 'ALL'; store.addLog("Reading all flash...", "warn"); }

      const unlisten = await listen('completed', async (event: any) => {
        if (event.payload.success) {
          const data = await readLocalFile(tempFile, 0, 0x10000); 
          currentTab.data = data;
          store.addLog(`Read success.`, "success");
        }
        unlisten();
      });

      await readFlash(store.port, parseInt(store.baudrate, 10), finalAddress, finalSize, tempFile);
    } catch (e: any) {
      store.isBusy = false; store.status = 'error'; store.addLog(`Read failed: ${e}`, "error");
    }
  } else if (currentTab.path) {
    try {
      let finalAddress = address.value.trim();
      if (!finalAddress || finalAddress === '0x') finalAddress = '0x0';
      let finalSize = size.value.trim();
      if (!finalSize || finalSize === '0x') finalSize = '0x400';
      const offset = parseInt(finalAddress, 16) || 0;
      const readSize = Math.min(parseInt(finalSize, 16) || 0x400, 0x100000);
      const data = await readLocalFile(currentTab.path, offset, readSize);
      currentTab.data = data;
      store.addLog(`Loaded file ${currentTab.name}`, "info");
    } catch (e: any) {
      store.addLog(`File read failed: ${e}`, "error");
    }
  }
};

const handleDownload = async () => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (!currentTab || currentTab.type !== 'file' || !currentTab.path) return;
  if (!store.port) { alert("Select Port!"); return; }

  store.clearLogs(); store.isBusy = true; store.status = 'connecting'; store.progress = 0;
  let finalAddress = address.value.trim();
  if (!finalAddress || finalAddress === '0x') finalAddress = '0x0';

  const payload: FlashStartPayload = {
    port: store.port, baud: parseInt(store.baudrate, 10), flash_mode: store.flashMode, flash_freq: store.flashFreq, flash_size: store.flashSize !== 'keep' ? store.flashSize : undefined, erase_before: false, items: [{ offset: finalAddress, file_path: currentTab.path }], extra_args: store.extraArgs
  };

  try { await startFlash(payload); } catch (err: any) { store.isBusy = false; store.status = 'error'; store.addLog(`Download failed: ${err}`, "error"); }
};

const handleMainAction = () => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (currentTab && currentTab.type === 'file') handleDownload(); else readFileData(false);
};

const handleDropdownAction = async (action: string) => {
  isDropdownOpen.value = false;
  if (action === 'read_all') readFileData(true);
  else if (action === 'save_file') {
    const currentTab = tabs.value.find(t => t.id === activeTabId.value);
    if (!currentTab || currentTab.data.length === 0) { alert("No data."); return; }
    try {
      const filePath = await save({ filters: [{ name: 'Binary', extensions: ['bin'] }], defaultPath: 'memory_dump.bin' });
      if (filePath) { await writeLocalFile(filePath, currentTab.data); store.addLog(`Saved to ${filePath}`, "success"); }
    } catch (e: any) { store.addLog(`Save failed: ${e}`, "error"); }
  }
};

const formatOptionLabel = (addr: string, label: string) => `${addr} (${label})`;
const updateAddressFromList = (e: Event) => {
  const val = (e.target as HTMLInputElement).value.split(' ')[0];
  if (val && val.startsWith('0x')) address.value = val;
};
</script>

<template>
  <div class="h-full flex flex-col bg-[#121212]" @click="isDropdownOpen = false">
    <!-- Tabs Header (MUI Style) -->
    <div class="flex bg-[#1e1e1e] border-b border-[#333333] overflow-x-auto custom-scrollbar shrink-0 shadow-sm relative">
      <div 
        v-for="tab in tabs" :key="tab.id" @click="activeTabId = tab.id"
        class="px-5 py-3 text-xs font-bold flex items-center space-x-3 cursor-pointer transition-colors border-b-2"
        :class="activeTabId === tab.id ? 'text-[#90caf9] bg-[#90caf9]/5 border-[#90caf9]' : 'text-[#9e9e9e] border-transparent hover:text-[#e0e0e0] hover:bg-[#2a2a2a] hover:border-[#444]'"
      >
        <span>{{ tab.name }}</span>
        <button v-if="tab.id !== 'device'" @click.stop="closeTab(tab.id)" class="hover:text-[#f44336] hover:bg-[#f44336]/20 rounded-full p-0.5 outline-none transition-colors">×</button>
      </div>
      
      <!-- Add Button -->
      <button @click="pickFile" class="px-4 text-[#b3b3b3] hover:text-[#90caf9] hover:bg-[#2a2a2a] transition-colors outline-none flex items-center justify-center">
        <Plus class="w-4 h-4" />
      </button>
    </div>

    <!-- Main Content -->
    <div class="flex-1 flex flex-col overflow-hidden bg-[#1e1e1e]">
      <!-- Toolbar -->
      <div class="flex flex-wrap items-center bg-[#1e1e1e] p-3 border-b border-[#333] shadow-sm space-x-6 shrink-0">
        <div class="flex items-center space-x-2 relative">
          <label class="text-[10px] font-bold uppercase tracking-wider text-[#9e9e9e]">Address</label>
          <input type="text" list="esp-addresses" v-model="address" @change="updateAddressFromList" class="w-44 bg-[#121212] border border-[#444] text-xs rounded-md px-2.5 py-1.5 outline-none focus:border-[#90caf9] font-mono text-[#e0e0e0]" spellcheck="false" />
          <datalist id="esp-addresses">
            <option :value="formatOptionLabel('0x1000', 'Bootloader')"></option>
            <option :value="formatOptionLabel('0x8000', 'Partition Table')"></option>
            <option :value="formatOptionLabel('0xe000', 'Boot App')"></option>
            <option :value="formatOptionLabel('0x10000', 'Firmware (ESP32)')"></option>
            <option :value="formatOptionLabel('0x0', 'Firmware (ESP8266)')"></option>
          </datalist>
        </div>

        <div class="flex items-center space-x-2">
          <label class="text-[10px] font-bold uppercase tracking-wider text-[#9e9e9e]">Size</label>
          <input type="text" v-model="size" class="w-24 bg-[#121212] border border-[#444] text-xs rounded-md px-2.5 py-1.5 outline-none focus:border-[#90caf9] font-mono text-[#e0e0e0]" spellcheck="false" />
        </div>
        
        <div class="flex items-center space-x-2">
          <label class="text-[10px] font-bold uppercase tracking-wider text-[#9e9e9e]">Width</label>
          <select v-model="dataWidth" class="w-24 bg-[#121212] border border-[#444] text-xs rounded-md px-2.5 py-1.5 outline-none focus:border-[#90caf9] text-[#e0e0e0] cursor-pointer">
            <option :value="8" class="bg-[#1e1e1e] text-[#e0e0e0]">8-bit</option>
            <option :value="16" class="bg-[#1e1e1e] text-[#e0e0e0]">16-bit</option>
            <option :value="32" class="bg-[#1e1e1e] text-[#e0e0e0]">32-bit</option>
          </select>
        </div>
        
        <div class="flex-1"></div>

        <!-- Combo Button MUI Style -->
        <div class="flex relative shadow-md rounded-md group">
          <button @click="handleMainAction" class="flex items-center px-5 py-1.5 bg-[#1976d2] hover:bg-[#1565c0] text-white text-xs font-bold uppercase tracking-wider transition-colors border-r border-[#115293] rounded-l-md outline-none focus:ring-2 focus:ring-[#90caf9]">
            <Download class="w-4 h-4 mr-2" /> {{ activeTabId !== 'device' ? 'Download' : 'Read' }}
          </button>
          <button v-if="activeTabId === 'device'" @click.stop="isDropdownOpen = !isDropdownOpen" class="px-2 py-1.5 bg-[#1976d2] hover:bg-[#1565c0] text-white transition-colors rounded-r-md outline-none focus:ring-2 focus:ring-[#90caf9]">
            <ChevronDown class="w-4 h-4" />
          </button>

          <div v-if="isDropdownOpen && activeTabId === 'device'" class="absolute top-full right-0 mt-1.5 w-40 bg-[#2a2a2a] border border-[#444] shadow-2xl rounded-md py-1 z-50 overflow-hidden">
            <button @click="handleDropdownAction('read_all')" class="w-full text-left px-4 py-2.5 text-xs text-[#e0e0e0] hover:bg-[#333] hover:text-[#90caf9] transition-colors font-bold">Read All</button>
            <button @click="handleDropdownAction('save_file')" class="w-full text-left px-4 py-2.5 text-xs text-[#e0e0e0] hover:bg-[#333] hover:text-[#90caf9] transition-colors font-bold">Save to file</button>
          </div>
        </div>
      </div>

      <!-- Hex Viewer Table -->
      <div class="flex-1 overflow-auto custom-scrollbar bg-[#121212]">
        <table class="w-full text-xs font-mono text-left whitespace-nowrap">
          <thead class="bg-[#2c2c2c] sticky top-0 z-10 shadow-sm border-b border-[#444] text-[#b3b3b3]">
            <tr>
              <th class="px-4 py-2 font-bold w-32 border-r border-[#444]">Address</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-bold text-center">0</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-bold text-center">4</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-bold text-center">8</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-bold text-center">C</th>
              <th v-if="dataWidth === 16" v-for="i in 8" :key="i" class="px-2 py-2 font-bold text-center">{{ ((i-1)*2).toString(16).toUpperCase() }}</th>
              <th v-if="dataWidth === 8" v-for="i in 16" :key="i" class="px-1 py-2 font-bold text-center">{{ (i-1).toString(16).toUpperCase() }}</th>
              <th class="px-4 py-2 font-bold border-l border-[#444]">ASCII</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[#222]">
            <tr v-for="(row, idx) in dataRows" :key="idx" class="hover:bg-[#1a232e] transition-colors" :class="idx % 2 === 0 ? 'bg-[#1e1e1e]' : 'bg-[#181818]'">
              <td class="px-4 py-1 text-[#90caf9] border-r border-[#333] font-bold">{{ row.addr }}</td>
              <td v-for="(word, i) in row.words" :key="i" class="px-2 py-1 text-center" :class="word === '00000000' || word === '0000' || word === '00' ? 'text-[#555]' : 'text-[#e0e0e0]'">
                {{ word }}
              </td>
              <td class="px-4 py-1 text-[#b3b3b3] border-l border-[#333] tracking-[0.2em]">{{ row.ascii }}</td>
            </tr>
            <tr v-if="dataRows.length === 0">
              <td colspan="10" class="text-center py-10 text-[#757575] italic">
                {{ store.isBusy ? 'Loading data...' : 'Click Read/Download to load memory.' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
