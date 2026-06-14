<script setup lang="ts">
import type { RouteLocationNormalizedLoaded } from 'vue-router'

function routeShellKey(route: RouteLocationNormalizedLoaded) {
  return route.matched[0]?.path || route.path
}
</script>

<template>
  <router-view v-slot="{ Component, route }">
    <transition name="mobile-route-fade">
      <component :is="Component" :key="routeShellKey(route)" />
    </transition>
  </router-view>
</template>
