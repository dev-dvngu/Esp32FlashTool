<script setup lang="ts">
import { useMainStore } from '../stores/main';
import { Menu, Database, Download, UserCircle } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import { getEsptoolVersion } from '../api/commands';
import { open as openUrl } from '@tauri-apps/plugin-shell';

import InterfaceSelector from '../components/ui/InterfaceSelector.vue';
import UartConfig from '../components/ui/UartConfig.vue';
import TargetInfo from '../components/ui/TargetInfo.vue';
import CircularProgress from '../components/ui/CircularProgress.vue';
import BottomPanel from './BottomPanel.vue';

const store = useMainStore();
const esptoolVer = ref('esptool');
const isSidebarExpanded = ref(false);

// Shortened labels for MUI style
const menuItems = [
  { id: 'memory', icon: Database, label: 'Memory' },
  { id: 'flashing', icon: Download, label: 'Flashing' },
];

const getActiveTitle = () => menuItems.find(i => i.id === store.activeTab)?.label || 'EspFlash Programmer';

onMounted(async () => {
  try {
    const ver = await getEsptoolVersion();
    const match = ver.match(/v\d+\.\d+(\.\d+)?/);
    if (match) esptoolVer.value = `esptool ${match[0]}`;
  } catch (e) { console.error(e); }
});

const handleOpenGithub = async () => {
  try {
    await openUrl('https://github.com/dev-dvngu');
  } catch (e) {
    console.error("Failed to open URL:", e);
    window.open('https://github.com/dev-dvngu', '_blank');
  }
};
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-[#121212] text-[#e0e0e0] font-sans overflow-hidden selection:bg-[#90caf9] selection:text-black">
    
    <!-- Header (MUI App Bar: Elevation 4) -->
    <header class="h-14 bg-[#1e1e1e] border-b border-[#333333] flex items-center justify-between px-4 z-30 shrink-0 shadow-md">
      <div class="flex items-center space-x-4">
        <button @click="isSidebarExpanded = !isSidebarExpanded" class="p-2 rounded-full hover:bg-[#333333] transition-colors focus:outline-none focus:ring-2 focus:ring-[#90caf9]">
          <Menu class="w-5 h-5 text-[#e0e0e0]" />
        </button>
        <span class="text-lg font-medium tracking-wide">{{ getActiveTitle() }}</span>
      </div>
      
      <div class="flex items-center space-x-6">
        <div class="flex flex-col items-end pr-4 border-r border-[#333333]">
          <span class="text-[9px] font-bold uppercase text-[#9e9e9e] leading-none mb-1">Status</span>
          <span class="text-xs font-bold" :class="store.isBusy ? 'text-[#ff9800]' : 'text-[#4caf50]'">
            {{ store.isBusy ? store.status + '...' : 'Ready' }}
          </span>
        </div>
        <div class="flex flex-col items-start">
          <span class="text-[9px] font-bold uppercase text-[#9e9e9e] leading-none mb-1">Core</span>
          <span class="text-xs font-medium text-[#e0e0e0]">{{ esptoolVer }}</span>
        </div>
      </div>
    </header>

    <div class="flex-1 flex overflow-hidden">
      <!-- Left Sidebar (MUI Drawer) -->
      <aside 
        class="bg-[#1e1e1e] flex flex-col border-r border-[#333333] z-20 transition-all duration-300 overflow-hidden shrink-0 shadow-lg"
        :class="isSidebarExpanded ? 'w-60' : 'w-[72px] items-center'"
      >
        <!-- Nav Items -->
        <nav class="flex-1 w-full py-4 px-2 space-y-2">
          <button 
            v-for="item in menuItems" 
            :key="item.id"
            @click="store.activeTab = item.id"
            class="flex items-center py-3 px-3 relative group transition-all w-full rounded-lg outline-none focus:ring-2 focus:ring-[#90caf9] overflow-hidden"
            :class="[
              store.activeTab === item.id ? 'bg-[#90caf9]/20 text-[#90caf9]' : 'text-[#b3b3b3] hover:bg-[#333333] hover:text-[#e0e0e0]',
              isSidebarExpanded ? 'justify-start' : 'justify-center'
            ]"
          >
            <!-- Highlight bar for active state (MUI indicator style) -->
            <div v-if="store.activeTab === item.id" class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-[#90caf9] rounded-r-md"></div>
            
            <component :is="item.icon" class="w-6 h-6 flex-shrink-0" :class="store.activeTab === item.id ? 'text-[#90caf9]' : ''" />
            <span v-if="isSidebarExpanded" class="ml-4 text-sm font-medium whitespace-nowrap">{{ item.label }}</span>
            <span v-if="!isSidebarExpanded" class="absolute left-16 bg-[#333333] text-[#e0e0e0] text-xs px-3 py-1.5 rounded-md opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap z-50 shadow-xl border border-[#444] transition-opacity">{{ item.label }}</span>
          </button>
        </nav>

        <!-- Bottom Info Section -->
        <div class="w-full border-t border-[#333333] p-3 flex flex-col" :class="isSidebarExpanded ? 'items-start' : 'items-center'">
          <button 
            @click="handleOpenGithub"
            class="w-full flex items-center py-3 px-3 rounded-lg hover:bg-[#333333] transition-all group relative focus:outline-none focus:ring-2 focus:ring-[#90caf9]"
            :class="isSidebarExpanded ? 'justify-start' : 'justify-center'"
          >
            <UserCircle class="w-6 h-6 shrink-0 text-[#9e9e9e] group-hover:text-[#e0e0e0] transition-colors" />
            
            <div v-if="isSidebarExpanded" class="ml-4 flex flex-col items-start overflow-hidden text-left">
              <span class="text-xs font-bold text-[#e0e0e0]">Dv Ngu</span>
              <span class="text-[10px] text-[#9e9e9e] truncate w-36 group-hover:text-[#90caf9] transition-colors">github.com/dev-dvngu</span>
            </div>

            <!-- Tooltip for collapsed mode -->
            <div v-if="!isSidebarExpanded" class="absolute left-16 bottom-1 bg-[#333333] text-[#e0e0e0] text-xs p-4 rounded-lg shadow-2xl border border-[#444] opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap z-50 transition-opacity">
              <p class="font-bold text-[#90caf9] border-b border-[#555] pb-2 mb-2">ABOUT DEVELOPER</p>
              <p class="font-bold text-[13px]">Dv Ngu</p>
              <p class="text-[#b3b3b3] mt-1">github.com/dev-dvngu</p>
              <p class="text-[#757575] mt-3 text-[9px] italic">© 2026 All rights reserved</p>
            </div>
          </button>
        </div>
      </aside>

      <!-- MUI DASHBOARD LAYER -->
      <div class="flex-1 flex overflow-hidden p-4 gap-4">
        <!-- Left Column -->
        <div class="flex-1 flex flex-col min-w-0 gap-4 overflow-hidden">
          <div class="flex-1 bg-[#1e1e1e] border border-[#333333] shadow-md overflow-hidden flex flex-col rounded-lg">
            <div class="flex-1 overflow-y-auto">
              <slot></slot>
            </div>
          </div>
          <!-- Log panel fixed height -->
          <div class="h-64 bg-[#1e1e1e] border border-[#333333] shadow-md shrink-0 rounded-lg overflow-hidden flex flex-col">
            <BottomPanel />
          </div>
        </div>

        <!-- Right Column (Cards) -->
        <div class="w-[340px] flex flex-col gap-4 shrink-0 overflow-hidden">
          <InterfaceSelector />
          <UartConfig />
          <TargetInfo class="flex-1" />
          <CircularProgress class="shrink-0 h-64" />
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* Reset specific scrollbars in components to use the global dark one */
.custom-scrollbar::-webkit-scrollbar { width: 8px; height: 8px; }
.custom-scrollbar::-webkit-scrollbar-track { background: #1e1e1e; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #424242; border-radius: 4px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #616161; }
</style>
