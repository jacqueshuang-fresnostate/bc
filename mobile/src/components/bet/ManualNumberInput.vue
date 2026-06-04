<script setup lang="ts">
defineProps<{ modelValue: string }>()
const emit = defineEmits<{ 'update:modelValue': [string] }>()
</script>

<template>
  <van-cell-group inset style="margin-top: 12px;">
    <van-field :model-value="modelValue" label="投注号码" placeholder="请输入投注号码" @update:model-value="value => emit('update:modelValue', String(value))" />
  </van-cell-group>
</template>
