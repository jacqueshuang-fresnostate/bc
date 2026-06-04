<script setup lang="ts">
defineOptions({ name: 'CountdownBadge' })
defineProps<{ text: string; label?: string }>()
</script>

<template>
  <div class="flex items-center gap-1.5 rounded-full bg-red-50 px-3 py-1 text-primary">
    <span class="text-xs">{{ label || '计时' }}</span>
    <span class="font-headline text-xs font-bold tracking-tighter">{{ text }}</span>
  </div>
</template>
