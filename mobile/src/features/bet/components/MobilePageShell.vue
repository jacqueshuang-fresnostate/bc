<script setup lang="ts">
withDefaults(defineProps<{ paddedBottom?: boolean }>(), { paddedBottom: true })
</script>

<template>
  <div class="min-h-screen bg-[#f9f9f9] text-[#1a1c1c]" :class="paddedBottom ? 'pb-40' : ''">
    <slot />
  </div>
</template>
