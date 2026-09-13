<script setup lang="ts">
import { Headphones } from "lucide-vue-next";

withDefaults(
  defineProps<{
    title?: string;
    desc?: string;
    icon?: typeof Headphones;
  }>(),
  {
    title: "这里空空的",
    desc: "还没有内容",
    icon: Headphones,
  }
);
</script>

<template>
  <div class="flex flex-col items-center justify-center py-20 text-center">
    <div class="w-16 h-16 rounded-2xl bg-bg-card border border-bg-border flex items-center justify-center text-muted mb-4">
      <component :is="icon" :size="28" />
    </div>
    <div class="text-sm font-medium text-white">{{ title }}</div>
    <div class="text-xs text-muted mt-1 max-w-sm">{{ desc }}</div>
    <slot />
  </div>
</template>
