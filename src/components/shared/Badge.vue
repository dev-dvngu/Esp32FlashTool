<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(defineProps<{
  variant?: 'info' | 'success' | 'warning' | 'error' | 'default';
}>(), {
  variant: 'default',
});

const variantClasses = computed(() => {
  switch (props.variant) {
    case 'success': return "bg-green-500/10 text-green-400 border-green-500/20";
    case 'warning': return "bg-yellow-500/10 text-yellow-400 border-yellow-500/20";
    case 'error': return "bg-red-500/10 text-red-400 border-red-500/20";
    case 'info': return "bg-blue-500/10 text-blue-400 border-blue-500/20";
    default: return "bg-gray-800 text-gray-300 border-gray-700";
  }
});
</script>

<template>
  <span 
    class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium border"
    :class="variantClasses"
  >
    <slot></slot>
  </span>
</template>
