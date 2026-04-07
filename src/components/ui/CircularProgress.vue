<script setup lang="ts">
import { useMainStore } from '../../stores/main';
const store = useMainStore();

/**
 * Viền Progress bao quanh hộp 340x256
 * Chu vi chính xác = (340 * 2) + (256 * 2) = 1192
 */
const strokeDasharray = 1192;
const strokeDashoffset = (p: number) => strokeDasharray - (p / 100) * strokeDasharray;

const pathD = `M 170 1 
  L 332 1 
  A 7 7 0 0 1 339 8 
  L 339 248 
  A 7 7 0 0 1 332 255 
  L 8 255 
  A 7 7 0 0 1 1 248 
  L 1 8 
  A 7 7 0 0 1 8 1 
  L 170 1 
  Z`;
</script>

<template>
  <div class="flex flex-col shrink-0 h-64 bg-[#1e1e1e] rounded-lg overflow-hidden relative select-none shadow-xl border border-transparent">
    
    <!-- SVG Layer (The Actual Border) -->
    <div class="absolute inset-0 z-0">
      <svg class="w-full h-full" viewBox="0 0 340 256" preserveAspectRatio="none">
        <path
          :d="pathD"
          stroke="#333333"
          stroke-width="2"
          fill="transparent"
        />
        <path
          :d="pathD"
          stroke="currentColor"
          stroke-width="3"
          fill="transparent"
          :stroke-dasharray="strokeDasharray"
          :stroke-dashoffset="strokeDashoffset(store.progress)"
          stroke-linecap="square"
          class="transition-all duration-500 ease-out"
          :class="[
            store.status === 'error' ? 'text-[#f44336]' : 'text-[#90caf9]',
            {'animate-pulse': store.isBusy}
          ]"
        />
      </svg>
    </div>

    <!-- Center Content -->
    <div class="flex-1 flex flex-col items-center justify-center relative z-10 p-6">
      <span class="text-[10px] font-black uppercase tracking-[0.5em] text-[#555] mb-4">Task Execution</span>
      
      <div class="flex items-baseline mb-2">
        <span class="text-7xl font-black font-mono text-white tracking-tighter drop-shadow-2xl">
          {{ store.progress }}
        </span>
        <span class="text-xl font-bold text-[#90caf9] ml-1 opacity-60">%</span>
      </div>
      
      <div class="flex flex-col items-center">
        <div class="px-4 py-1.5 rounded bg-[#121212] border border-[#333] shadow-inner mb-4 flex items-center space-x-3">
          <div class="w-2 h-2 rounded-full" :class="store.isBusy ? 'bg-[#4caf50] animate-ping' : 'bg-[#444]'"></div>
          <span class="text-xs font-black uppercase tracking-widest" :class="[
             store.status === 'error' ? 'text-[#f44336]' : (store.isBusy ? 'text-[#90caf9]' : 'text-slate-400')
          ]">
            {{ store.isBusy ? store.status : (store.status === 'error' ? 'Failed' : 'System Ready') }}
          </span>
        </div>
        
        <!-- Animated Timer -->
        <div class="bg-[#252525] px-3 py-1 rounded-full border border-[#333] flex items-center space-x-2">
          <span class="text-[9px] font-bold text-slate-500 font-mono tracking-widest uppercase opacity-70">Elapsed</span>
          <span class="text-[10px] font-bold text-blue-200/70 font-mono tracking-tighter transition-all" :class="{'text-[#a0d030] scale-110': store.isBusy}">
            {{ store.elapsedTime }}
          </span>
        </div>
      </div>
    </div>

  </div>
</template>
