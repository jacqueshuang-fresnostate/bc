<script setup lang="ts">
import DynamicBetPage from '../features/bet/components/DynamicBetPage.vue'

defineProps<{ wsMessage?: any }>()
</script>

<template>
  <DynamicBetPage :ws-message="wsMessage" />
</template>
