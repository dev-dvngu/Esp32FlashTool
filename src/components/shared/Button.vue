<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  block?: boolean;
}>(), {
  variant: 'primary',
  size: 'md',
  disabled: false,
  block: false,
});

const baseClasses = "inline-flex items-center justify-center font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-900 rounded-md disabled:opacity-50 disabled:cursor-not-allowed";

const sizeClasses = computed(() => {
  switch (props.size) {
    case 'sm': return "px-3 py-1.5 text-sm";
    case 'lg': return "px-6 py-3 text-lg";
    default: return "px-4 py-2 text-base";
  }
});

const variantClasses = computed(() => {
  switch (props.variant) {
    case 'secondary': return "bg-gray-700 text-gray-200 hover:bg-gray-600 focus:ring-gray-500";
    case 'danger': return "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500";
    case 'ghost': return "bg-transparent text-gray-300 hover:bg-gray-800 focus:ring-gray-600";
    default: return "bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500";
  }
});

const blockClass = computed(() => props.block ? "w-full" : "");

</script>

<template>
  <button 
    :class="[baseClasses, sizeClasses, variantClasses, blockClass]" 
    :disabled="disabled"
    @click="$emit('click')"
  >
    <slot></slot>
  </button>
</template>
