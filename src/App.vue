<script setup lang="ts">
import { onMounted } from 'vue';
import { useMainStore } from './stores/main';
import MainLayout from './layouts/MainLayout.vue';
import FlashingWorkspace from './features/flashing/FlashingWorkspace.vue';
import MemoryWorkspace from './features/memory/MemoryWorkspace.vue';

const store = useMainStore();

onMounted(() => {
  store.loadSettings();
  store.initListeners();
});
</script>

<template>
  <MainLayout>
    <FlashingWorkspace v-if="store.activeTab === 'flashing'" />
    <MemoryWorkspace v-else-if="store.activeTab === 'memory'" />
    <div v-else class="flex items-center justify-center h-full text-slate-400">
      Advanced Commands coming soon...
    </div>
  </MainLayout>
</template>
