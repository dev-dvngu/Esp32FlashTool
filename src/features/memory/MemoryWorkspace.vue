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
  { id: 'device', name: 'Device memory', type: 'device', data: [] },
]);

const activeTabId = ref('device');

const address = ref('0x0');
const size = ref('0x400');
const dataWidth = ref(32); // 8, 16, 32
const isDropdownOpen = ref(false);

// TỐI ƯU: Chỉ hiển thị tối đa 1024 dòng (16KB) để tránh treo trình duyệt
const MAX_DISPLAY_SIZE = 0x4000; 

const dataRows = computed(() => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (!currentTab || !currentTab.data.length) return [];
  
  const rows = [];
  const bytesPerRow = 16;
  
  // Chỉ lấy một phần dữ liệu để hiển thị nếu dữ liệu quá lớn
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
    if (activeTabId.value === id) {
      activeTabId.value = 'device';
    }
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
    tabs.value.push({
      id,
      name: fileName,
      type: 'file',
      path: selected,
      data: []
    });
    activeTabId.value = id;
    await readFileData();
  }
};

const readFileData = async (readAll: boolean = false) => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (!currentTab) return;

  if (currentTab.type === 'device') {
    if (!store.port) {
      alert("Please select a serial port first!");
      return;
    }

    try {
      const tDir = await tempDir();
      const tempFile = await join(tDir, 'esp_read_temp.bin');
      
      store.clearLogs();
      store.isBusy = true;
      store.status = 'connecting';
      store.progress = 0;

      // Chuẩn hóa Address và Size
      let finalAddress = address.value.trim();
      if (!finalAddress || finalAddress === '0x') finalAddress = '0x0';
      
      let finalSize = size.value.trim();
      if (!finalSize || finalSize === '0x') finalSize = '0x400';
      
      if (readAll) {
        finalAddress = '0x0';
        finalSize = 'ALL';
        store.addLog("Đang tải toàn bộ dữ liệu Flash (Read All). Vui lòng đợi...", "warn");
      }

      const unlisten = await listen('completed', async (event: any) => {
        if (event.payload.success) {
          // Khi nạp xong, ta đọc tối đa 64KB đầu tiên để hiển thị lên UI, tránh crash
          const data = await readLocalFile(tempFile, 0, 0x10000); 
          currentTab.data = data;
          store.addLog(`Đã đọc ${readAll ? 'toàn bộ' : data.length + ' bytes'} thành công.`, "success");
          if (readAll) {
             store.addLog(`Dữ liệu gốc được lưu tạm tại: ${tempFile}`, "info");
          }
        }
        unlisten();
      });

      await readFlash(store.port, parseInt(store.baudrate, 10), finalAddress, finalSize, tempFile);
    } catch (e: any) {
      store.isBusy = false;
      store.status = 'error';
      store.addLog(`Read Flash failed: ${e}`, "error");
    }
  } else if (currentTab.path) {
    try {
      let finalAddress = address.value.trim();
      if (!finalAddress || finalAddress === '0x') finalAddress = '0x0';
      let finalSize = size.value.trim();
      if (!finalSize || finalSize === '0x') finalSize = '0x400';

      const offset = parseInt(finalAddress, 16) || 0;
      const readSize = Math.min(parseInt(finalSize, 16) || 0x400, 0x100000); // Giới hạn 1MB cho file local
      const data = await readLocalFile(currentTab.path, offset, readSize);
      currentTab.data = data;
      store.addLog(`Loaded ${data.length} bytes from ${currentTab.name}`, "info");
    } catch (e: any) {
      store.addLog(`Failed to read file: ${e}`, "error");
    }
  }
};

const handleDownload = async () => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (!currentTab || currentTab.type !== 'file' || !currentTab.path) return;

  if (!store.port) {
    alert("Please select a serial port first!");
    return;
  }

  store.clearLogs();
  store.isBusy = true;
  store.status = 'connecting';
  store.progress = 0;

  let finalAddress = address.value.trim();
  if (!finalAddress || finalAddress === '0x') finalAddress = '0x0';

  const payload: FlashStartPayload = {
    port: store.port,
    baud: parseInt(store.baudrate, 10),
    flash_mode: store.flashMode,
    flash_freq: store.flashFreq,
    flash_size: store.flashSize !== 'keep' ? store.flashSize : undefined,
    erase_before: false,
    verify_after: true,
    items: [{ offset: finalAddress, file_path: currentTab.path }],
    extra_args: store.extraArgs
  };

  try {
    await startFlash(payload);
  } catch (err: any) {
    store.isBusy = false;
    store.status = 'error';
    store.addLog(`Download failed: ${err}`, "error");
  }
};

const handleMainAction = () => {
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (currentTab && currentTab.type === 'file') {
    handleDownload();
  } else {
    readFileData(false);
  }
};

const handleDropdownAction = async (action: string) => {
  isDropdownOpen.value = false;
  if (action === 'read_all') {
    readFileData(true);
  } else if (action === 'save_file') {
    const currentTab = tabs.value.find(t => t.id === activeTabId.value);
    if (!currentTab || currentTab.data.length === 0) {
      alert("No data to save. Please 'Read' first.");
      return;
    }
    
    try {
      const filePath = await save({
        filters: [{ name: 'Binary', extensions: ['bin'] }],
        defaultPath: 'memory_dump.bin'
      });
      if (filePath) {
        await writeLocalFile(filePath, currentTab.data);
        store.addLog(`Saved ${currentTab.data.length} bytes to ${filePath}`, "success");
      }
    } catch (e: any) {
      store.addLog(`Save failed: ${e}`, "error");
    }
  }
};

const formatOptionLabel = (addr: string, label: string) => {
  return `${addr} (${label})`;
};

const updateAddressFromList = (e: Event) => {
  const target = e.target as HTMLInputElement;
  const val = target.value.split(' ')[0];
  if (val && val.startsWith('0x')) {
    address.value = val;
  }
};
</script>

<template>
  <div class="h-full flex flex-col bg-white" @click="isDropdownOpen = false">
    <!-- Tabs Header -->
    <div class="flex items-end bg-white pt-2 px-2 border-b border-[#2b90d9] overflow-x-auto custom-scrollbar">
      <div 
        v-for="tab in tabs"
        :key="tab.id"
        @click="activeTabId = tab.id"
        class="px-4 py-1.5 text-sm flex items-center space-x-3 rounded-t-md relative top-[1px] cursor-pointer border-t border-x min-w-max"
        :class="activeTabId === tab.id 
          ? 'bg-[#002052] text-white border-[#002052]' 
          : 'bg-slate-100 text-slate-600 hover:bg-slate-200 border-slate-300'"
      >
        <span>{{ tab.name }}</span>
        <button v-if="tab.id !== 'device'" @click.stop="closeTab(tab.id)" class="hover:text-red-400 font-bold ml-2">×</button>
      </div>
      
      <button @click="pickFile" class="bg-[#002052] text-white px-3 py-1.5 ml-1 text-sm rounded-t-md hover:bg-[#003070] relative top-[1px]">
        <Plus class="w-4 h-4" />
      </button>
    </div>

    <!-- Main Content -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Toolbar -->
      <div class="flex flex-wrap items-center bg-[#f0f2f5] p-2 border-b border-slate-300 shadow-sm space-x-4">
        
        <div class="flex items-center space-x-2 relative">
          <label class="text-xs font-semibold text-slate-600">Address</label>
          <input 
            type="text" 
            list="esp-addresses" 
            v-model="address" 
            @change="updateAddressFromList"
            class="w-48 px-2 py-1 text-xs border border-slate-300 outline-none focus:border-[#2b90d9] font-mono" 
            spellcheck="false" 
          />
          <datalist id="esp-addresses">
            <option :value="formatOptionLabel('0x1000', 'Bootloader')"></option>
            <option :value="formatOptionLabel('0x8000', 'Partition Table')"></option>
            <option :value="formatOptionLabel('0xe000', 'Boot App')"></option>
            <option :value="formatOptionLabel('0x10000', 'Firmware (ESP32)')"></option>
            <option :value="formatOptionLabel('0x0', 'Firmware (ESP8266)')"></option>
          </datalist>
        </div>

        <div class="flex items-center space-x-2">
          <label class="text-xs font-semibold text-slate-600">Size</label>
          <input type="text" v-model="size" class="w-24 px-2 py-1 text-xs border border-slate-300 outline-none focus:border-[#2b90d9] font-mono" spellcheck="false" />
        </div>
        
        <div class="flex items-center space-x-2">
          <label class="text-xs font-semibold text-slate-600">Data width</label>
          <select v-model="dataWidth" class="w-24 px-2 py-1 text-xs border border-slate-300 outline-none focus:border-[#2b90d9] bg-white">
            <option :value="8">8-bit</option>
            <option :value="16">16-bit</option>
            <option :value="32">32-bit</option>
          </select>
        </div>
        
        <div class="flex-1"></div>

        <!-- Combo Button -->
        <div class="flex relative shadow-sm rounded-sm">
          <button 
            @click="handleMainAction" 
            class="flex items-center px-4 py-1.5 bg-[#2b90d9] hover:bg-[#207bbf] text-white text-xs font-bold transition-colors border-r border-[#1a6fa8] rounded-l-sm shadow-inner"
          >
            <Download class="w-4 h-4 mr-1.5" /> 
            {{ activeTabId !== 'device' ? 'Download' : 'Read' }}
          </button>
          
          <button 
            v-if="activeTabId === 'device'"
            @click.stop="isDropdownOpen = !isDropdownOpen" 
            class="px-2 py-1.5 bg-[#2b90d9] hover:bg-[#207bbf] text-white transition-colors rounded-r-sm shadow-inner"
          >
            <ChevronDown class="w-4 h-4" />
          </button>

          <!-- Dropdown Menu -->
          <div v-if="isDropdownOpen && activeTabId === 'device'" class="absolute top-full right-0 mt-1 w-32 bg-white border border-slate-300 shadow-xl rounded-sm py-1 z-50">
            <button @click="handleDropdownAction('read_all')" class="w-full text-left px-4 py-2 text-xs text-slate-700 hover:bg-blue-50 transition-colors font-semibold">
              Read All
            </button>
            <button @click="handleDropdownAction('save_file')" class="w-full text-left px-4 py-2 text-xs text-slate-700 hover:bg-blue-50 transition-colors font-semibold">
              Save to file
            </button>
          </div>
        </div>
      </div>

      <!-- Hex Viewer Table -->
      <div class="flex-1 overflow-auto custom-scrollbar bg-white">
        <table class="w-full text-xs font-mono text-left whitespace-nowrap">
          <thead class="bg-slate-100 sticky top-0 border-b border-slate-300 shadow-sm z-10">
            <tr>
              <th class="px-4 py-2 font-semibold text-slate-700 w-32 border-r border-slate-200">Address</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-semibold text-slate-700 text-center">0</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-semibold text-slate-700 text-center">4</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-semibold text-slate-700 text-center">8</th>
              <th v-if="dataWidth === 32" class="px-4 py-2 font-semibold text-slate-700 text-center">C</th>
              
              <th v-if="dataWidth === 16" v-for="i in 8" :key="i" class="px-2 py-2 font-semibold text-slate-700 text-center">{{ ((i-1)*2).toString(16).toUpperCase() }}</th>
              <th v-if="dataWidth === 8" v-for="i in 16" :key="i" class="px-1 py-2 font-semibold text-slate-700 text-center">{{ (i-1).toString(16).toUpperCase() }}</th>

              <th class="px-4 py-2 font-semibold text-slate-700 border-l border-slate-200">ASCII</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, idx) in dataRows" :key="idx" class="hover:bg-blue-50" :class="idx % 2 === 0 ? 'bg-white' : 'bg-[#f8f9fa]'">
              <td class="px-4 py-1 text-slate-600 border-r border-slate-200 font-bold">{{ row.addr }}</td>
              <td v-for="(word, i) in row.words" :key="i" class="px-2 py-1 text-center" :class="word === '00000000' || word === '0000' || word === '00' ? 'text-slate-300' : 'text-[#002052]'">
                {{ word }}
              </td>
              <td class="px-4 py-1 text-slate-500 border-l border-slate-200 tracking-[0.2em]">{{ row.ascii }}</td>
            </tr>
            <tr v-if="dataRows.length === 0">
              <td colspan="10" class="text-center py-10 text-slate-400 italic">
                {{ store.isBusy ? 'Loading data...' : 'Click Read/Download to load memory.' }}
              </td>
            </tr>
            <tr v-if="activeTabId === 'device' && store.status === 'done' && dataRows.length > 0">
               <td colspan="10" class="text-center py-4 text-slate-400 text-[10px] bg-slate-50 italic">
                 Displaying first 16KB of data. Use 'Save to file' to export full flash content.
               </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<style scoped>
</style>
