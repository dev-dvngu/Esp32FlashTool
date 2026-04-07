<script setup lang="ts">
import { useMainStore } from '../../stores/main';
const store = useMainStore();

// Tính toán cho r = 80
const strokeDasharray = 502.65;
const strokeDashoffset = (p: number) => strokeDasharray - (p / 100) * strokeDasharray;
</script>

<template>
  <div class="flex flex-col shrink-0 h-64 shadow-sm border border-[#004bb0] rounded-sm bg-[#001840] overflow-hidden relative select-none">
    
    <!-- No Header, direct progress content -->
    <div class="flex-1 flex flex-col items-center justify-center py-2 relative">
      
      <!-- Large Circular Progress (r=80) -->
      <div class="relative w-48 h-48 flex items-center justify-center">
        <svg class="w-48 h-48 transform -rotate-90 drop-shadow-md" viewBox="0 0 200 200">
          <!-- Background Track -->
          <circle
            cx="100" cy="100" r="80"
            stroke="currentColor"
            stroke-width="10"
            fill="transparent"
            class="text-[#002a60]"
          />
          <!-- Progress Arc -->
          <circle
            cx="100" cy="100" r="80"
            stroke="currentColor"
            stroke-width="10"
            fill="transparent"
            :stroke-dasharray="strokeDasharray"
            :stroke-dashoffset="strokeDashoffset(store.progress)"
            stroke-linecap="round"
            class="text-[#a0d030] transition-all duration-500 ease-out"
            :class="{'animate-pulse': store.isBusy}"
          />
        </svg>
        
        <!-- Text Information in Center -->
        <div class="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <div class="flex items-baseline mb-0.5">
            <span class="text-3xl font-black font-mono text-white tracking-tighter drop-shadow-sm">{{ store.progress }}</span>
            <span class="text-sm font-bold text-slate-400 ml-0.5">%</span>
          </div>
          
          <div class="flex flex-col items-center">
            <span class="text-[9px] font-black uppercase tracking-widest leading-none" :class="store.isBusy ? 'text-[#a0d030]' : 'text-slate-500'">
              {{ store.isBusy ? store.status : 'Ready' }}
            </span>
            
            <!-- Compact Elapsed Time -->
            <div class="mt-2 bg-white/5 px-2 py-0.5 rounded-full border border-white/5 shadow-inner">
              <span class="text-[8px] font-bold text-slate-400 font-mono">00:00:00</span>
            </div>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<style scoped>
</style>
