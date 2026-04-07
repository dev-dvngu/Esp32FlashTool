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
      class="w-full bg-[#121212] border text-xs rounded-md px-2.5 py-1.5 outline-none transition-colors font-mono text-[#e0e0e0] placeholder-[#757575]"
      :class="[
        isValid 
          ? 'border-[#444] focus:border-[#90caf9] focus:ring-1 focus:ring-[#90caf9]' 
          : 'border-[#f44336] focus:border-[#f44336] focus:ring-1 focus:ring-[#f44336]'
      ]"
      spellcheck="false"
    />
  </div>
</template>
