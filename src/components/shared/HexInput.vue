<script setup lang="ts">
import { ref, watch } from 'vue';

const props = defineProps<{
  modelValue: string;
  placeholder?: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
}>();

const internalValue = ref(props.modelValue);
const isValid = ref(true);

const validateHex = (val: string) => {
  if (!val) return true;
  return /^0x[0-9A-Fa-f]+$/i.test(val);
};

watch(() => props.modelValue, (newVal) => {
  internalValue.value = newVal;
  isValid.value = validateHex(newVal);
});

const onInput = (e: Event) => {
  const target = e.target as HTMLInputElement;
  const val = target.value.trim();
  internalValue.value = val;
  isValid.value = validateHex(val);
  emit('update:modelValue', val);
};
</script>

<template>
  <div class="relative">
    <input 
      type="text" 
      :value="internalValue"
      @input="onInput"
      :placeholder="placeholder || '0x1000'"
      class="w-full bg-white border text-sm rounded-sm px-2 py-1 outline-none transition-colors font-mono"
      :class="[
        isValid 
          ? 'border-slate-300 text-slate-800 focus:border-[#2b90d9]' 
          : 'border-red-500 text-red-600 focus:border-red-500'
      ]"
      spellcheck="false"
    />
  </div>
</template>
