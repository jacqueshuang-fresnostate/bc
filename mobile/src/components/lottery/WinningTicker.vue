<script setup lang="ts">
defineProps<{ tickerItems: string[] }>()
</script>

<template>
  <section class="winning-ticker flex items-center gap-3 overflow-hidden rounded-xl bg-surface-container-lowest px-4 py-3 shadow-sm">
    <div class="flex items-center gap-2 text-primary">
      <span class="text-sm">公告</span>
      <span class="font-headline text-[10px] font-bold tracking-tighter">实时中奖</span>
    </div>
    <div class="flex-1 overflow-hidden">
      <div class="flex gap-8 whitespace-nowrap text-xs font-medium text-on-surface-variant">
        <template v-for="item in tickerItems" :key="item">
          <span>{{ item }}</span>
          <span class="opacity-30">|</span>
        </template>
      </div>
    </div>
  </section>
</template>
