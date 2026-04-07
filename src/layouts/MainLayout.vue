<script setup lang="ts">
import { useMainStore } from '../stores/main';
import { Menu, Database, Download, UserCircle } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import { getEsptoolVersion } from '../api/commands';
// Trong Tauri v2, dùng hàm open từ @tauri-apps/api/core hoặc plugin-opener tùy cấu hình
// Để an toàn và đơn giản, ta sẽ dùng window.open nếu được phép, hoặc sửa lại import đúng
import { open as openUrl } from '@tauri-apps/plugin-shell';

import InterfaceSelector from '../components/ui/InterfaceSelector.vue';
import UartConfig from '../components/ui/UartConfig.vue';
import TargetInfo from '../components/ui/TargetInfo.vue';
import CircularProgress from '../components/ui/CircularProgress.vue';
import BottomPanel from './BottomPanel.vue';

const store = useMainStore();
const esptoolVer = ref('esptool');
const isSidebarExpanded = ref(false);

const menuItems = [
  { id: 'memory', icon: Database, label: 'Memory & File editing' },
  { id: 'flashing', icon: Download, label: 'Erasing & Programming' },
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
    // Fallback if plugin fails
    window.open('https://github.com/dev-dvngu', '_blank');
  }
};
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-[#f0f2f5] text-slate-800 font-sans overflow-hidden">
    
    <!-- Header -->
    <header class="h-12 bg-[#002052] text-white flex items-center justify-between px-4 z-30 shrink-0 shadow-md">
      <div class="flex items-center space-x-4">
        <Menu @click="isSidebarExpanded = !isSidebarExpanded" class="w-6 h-6 cursor-pointer text-white hover:text-blue-300 transition-colors" />
        <span class="text-lg font-normal tracking-wide uppercase">{{ getActiveTitle() }}</span>
      </div>
      
      <div class="flex items-center space-x-6">
        <div class="flex flex-col items-end border-r border-slate-700 pr-4">
          <span class="text-[9px] font-bold uppercase text-slate-400 leading-none">Operation Status</span>
          <span class="text-xs font-bold" :class="store.isBusy ? 'text-[#a0d030]' : 'text-blue-300'">
            {{ store.isBusy ? store.status + '...' : 'System Ready' }}
          </span>
        </div>
        <div class="flex flex-col items-start">
          <span class="text-[9px] font-bold uppercase text-slate-400 leading-none">Core Version</span>
          <span class="text-xs font-bold text-white">{{ esptoolVer }}</span>
        </div>
      </div>
    </header>

    <div class="flex-1 flex overflow-hidden">
      <!-- Left Sidebar -->
      <aside 
        class="bg-[#002052] flex flex-col border-t border-[#001840] z-20 transition-all duration-300 overflow-hidden shrink-0 shadow-xl"
        :class="isSidebarExpanded ? 'w-56' : 'w-12 items-center'"
      >
        <!-- Nav Items -->
        <nav class="flex-1 w-full py-2">
          <button 
            v-for="item in menuItems" 
            :key="item.id"
            @click="store.activeTab = item.id"
            class="flex items-center py-3 px-3 relative group transition-colors w-full border-l-2"
            :class="[
              store.activeTab === item.id ? 'bg-[#0050a0] border-[#a0d030]' : 'hover:bg-[#003070] border-transparent',
              isSidebarExpanded ? 'justify-start' : 'justify-center'
            ]"
          >
            <component :is="item.icon" class="w-6 h-6 flex-shrink-0" :class="store.activeTab === item.id ? 'text-white' : 'text-[#a0d030]'" />
            <span v-if="isSidebarExpanded" class="ml-3 text-sm font-medium text-slate-200 whitespace-nowrap">{{ item.label }}</span>
            <span v-if="!isSidebarExpanded" class="absolute left-14 bg-slate-800 text-white text-[10px] px-2 py-1 rounded opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap z-50 shadow-lg">{{ item.label }}</span>
          </button>
        </nav>

        <!-- Bottom Info Section -->
        <div class="w-full border-t border-[#003070] bg-[#001840]/50">
          <button 
            @click="handleOpenGithub"
            class="w-full flex items-center py-4 px-3 hover:bg-[#003070] transition-all group relative"
            :class="isSidebarExpanded ? 'justify-start' : 'justify-center'"
          >
            <UserCircle class="w-6 h-6 shrink-0 text-[#a0d030] group-hover:scale-110 transition-transform" />
            
            <div v-if="isSidebarExpanded" class="ml-3 flex flex-col items-start overflow-hidden">
              <span class="text-[11px] font-black text-white uppercase tracking-tighter">Dv Ngu</span>
              <span class="text-[9px] text-slate-400 truncate w-32">github.com/dev-dvngu</span>
            </div>

            <!-- Tooltip for collapsed mode -->
            <div v-if="!isSidebarExpanded" class="absolute left-14 bottom-2 bg-white text-slate-900 text-[10px] p-3 rounded shadow-2xl border border-slate-200 opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap z-50 transition-opacity">
              <p class="font-black text-[#002052] border-b border-slate-100 pb-1 mb-1">DEVELOPER INFO</p>
              <p class="font-bold">Dv Ngu</p>
              <p class="text-blue-600">github.com/dev-dvngu</p>
              <p class="text-slate-400 mt-2 text-[8px] italic">© 2026 All rights reserved</p>
            </div>
          </button>
        </div>
      </aside>

      <!-- UNIFIED DASHBOARD LAYER -->
      <div class="flex-1 flex overflow-hidden p-2 gap-2">
        <div class="flex-1 flex flex-col min-w-0 gap-2 overflow-hidden">
          <div class="flex-1 bg-white border border-slate-300 shadow-sm overflow-hidden flex flex-col rounded-sm">
            <div class="flex-1 overflow-y-auto custom-scrollbar">
              <slot></slot>
            </div>
          </div>
          <div class="h-64 bg-white border border-slate-300 shadow-sm shrink-0 rounded-sm overflow-hidden text-slate-900">
            <BottomPanel />
          </div>
        </div>

        <div class="w-80 flex flex-col gap-2 shrink-0 overflow-hidden">
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
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 3px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
