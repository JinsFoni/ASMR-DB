<script setup lang="ts">
import { Headphones } from "lucide-vue-next";

// 注意：icon 不能作为 prop default（withDefaults）——lucide 图标是函数式组件，
// Vue 会把「函数类型的 default」当工厂函数调用，导致渲染崩溃。
// 因此 icon 只声明为可选 prop，缺省时在渲染处回退到 Headphones。
withDefaults(
  defineProps<{
    title?: string;
    desc?: string;
    icon?: typeof Headphones;
  }>(),
  {
    title: "这里空空的",
    desc: "还没有内容",
  }
);
</script>

<template>
  <div class="flex flex-col items-center justify-center py-20 text-center">
    <div class="w-16 h-16 rounded-2xl bg-bg-card border border-bg-border flex items-center justify-center text-muted mb-4">
      <component :is="icon ?? Headphones" :size="28" />
    </div>
    <div class="text-sm font-medium text-white">{{ title }}</div>
    <div class="text-xs text-muted mt-1 max-w-sm">{{ desc }}</div>
    <slot />
  </div>
</template>
