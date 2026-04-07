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
  <div class="flex flex-col h-full bg-white relative">
    <!-- Log Header -->
    <div class="flex items-center justify-between px-3 py-1 bg-slate-100 border-b border-slate-200">
      <span class="text-sm font-semibold text-slate-800">Log</span>
      
      <div class="flex items-center space-x-4 text-xs text-slate-700">
        <label class="flex items-center space-x-1 cursor-pointer">
          <input type="checkbox" checked class="w-3 h-3">
          <span>Auto Scroll</span>
        </label>
      </div>
    </div>

    <!-- Log Content Area -->
    <div class="flex flex-1 overflow-hidden relative">
      <div ref="scrollContainer" class="flex-1 overflow-y-auto p-2 font-mono text-[12px] leading-relaxed custom-scrollbar text-blue-700 bg-white">
        <div v-for="(log, idx) in store.logs" :key="idx" class="whitespace-pre-wrap break-all border-b border-slate-50 py-0.5"
             :class="{
               'text-blue-700': log.level === 'info',
               'text-amber-600': log.level === 'warn',
               'text-red-600 font-bold': log.level === 'error',
               'text-green-700 font-bold': log.level === 'success'
             }">
          <span class="text-slate-400 mr-2">{{ log.timestamp }} :</span> {{ log.text }}
        </div>
      </div>

      <!-- Action icons on the right side of log -->
      <div class="w-10 border-l border-slate-200 flex flex-col items-center py-2 space-y-4 bg-slate-50 relative z-10">
        <button @click="store.clearLogs()" class="text-[#2b90d9] hover:text-blue-800 transition-colors" title="Clear logs">
          <Eraser class="w-5 h-5" />
        </button>
      </div>
    </div>
  </div>
</template>
