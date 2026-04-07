<script setup lang="ts">
defineProps<{
  title?: string;
  noPadding?: boolean;
}>();
</script>

<template>
  <div class="bg-gray-800 border border-gray-700 rounded-lg overflow-hidden flex flex-col">
    <div v-if="title || $slots.header" class="px-4 py-3 border-b border-gray-700 bg-gray-800/50 flex items-center justify-between">
      <h3 v-if="title" class="text-sm font-semibold text-gray-200">{{ title }}</h3>
      <slot name="header"></slot>
    </div>
    <div :class="noPadding ? '' : 'p-4'" class="flex-1">
      <slot></slot>
    </div>
    <div v-if="$slots.footer" class="px-4 py-3 border-t border-gray-700 bg-gray-800/50">
      <slot name="footer"></slot>
    </div>
  </div>
</template>
