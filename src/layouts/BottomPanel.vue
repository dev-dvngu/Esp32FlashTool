<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useMainStore } from '../stores/main';
import { Eraser } from 'lucide-vue-next';

const store = useMainStore();
const scrollContainer = ref<HTMLElement | null>(null);

watch(() => store.logs.length, () => {
  if (scrollContainer.value) {
    nextTick(() => {
      scrollContainer.value!.scrollTop = scrollContainer.value!.scrollHeight;
    });
  }
});
</script>

<template>
  <div class="flex flex-col h-full bg-[#1e1e1e] relative">
    <!-- Log Header -->
    <div class="flex items-center justify-between px-4 py-2 bg-[#232323] border-b border-[#333333]">
      <span class="text-xs font-bold text-[#e0e0e0] uppercase tracking-wider">Log Console</span>
      
      <div class="flex items-center space-x-6 text-[11px] text-[#b3b3b3]">
        <label class="flex items-center space-x-2 cursor-pointer hover:text-[#e0e0e0] transition-colors group">
          <div class="relative flex items-center justify-center">
             <input type="checkbox" checked class="appearance-none w-3.5 h-3.5 rounded border border-[#555] checked:bg-[#90caf9] checked:border-[#90caf9] cursor-pointer transition-all peer">
             <svg class="absolute w-2.5 h-2.5 text-[#121212] opacity-0 peer-checked:opacity-100 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
          </div>
          <span>Live Update</span>
        </label>
        
        <button @click="store.clearLogs()" class="text-[#9e9e9e] hover:text-[#f44336] transition-colors p-2 rounded-md hover:bg-[#333] outline-none focus:ring-2 focus:ring-[#f44336]" title="Clear logs">
          <Eraser class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Log Content Area -->
    <div class="flex flex-1 overflow-hidden relative">
      <div ref="scrollContainer" class="flex-1 overflow-y-auto p-3 font-mono text-[12px] leading-relaxed custom-scrollbar bg-[#000000] text-[#e0e0e0] selection:bg-[#90caf9]/30">
        <div v-for="(log, idx) in store.logs" :key="idx" class="whitespace-pre-wrap break-all py-0.5 flex"
             :class="{
               'text-[#90caf9]': log.level === 'info',
               'text-[#ff9800]': log.level === 'warn',
               'text-[#f44336]': log.level === 'error',
               'text-[#4caf50]': log.level === 'success'
             }">
          <span class="text-[#555] mr-3 shrink-0 select-none">{{ log.timestamp }}</span>
          <span>{{ log.text }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
