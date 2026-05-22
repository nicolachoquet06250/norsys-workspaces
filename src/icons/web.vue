<script setup lang="ts">
import {StandardLonghandProperties} from "csstype";

defineProps<{
  width: StandardLonghandProperties["width"]
  height: StandardLonghandProperties["height"]
}>()
</script>

<template>
  <svg :width="width" :height="height" viewBox="0 0 60 60" fill="none">
    <path
        d="M30,53.5c-6.171,0-9.5-12.108-9.5-23.5S23.829,6.5,30,6.5,39.5,18.608,39.5,30,36.171,53.5,30,53.5Zm0-44c-2.714,0-6.5,7.8-6.5,20.5S27.286,50.5,30,50.5s6.5-7.8,6.5-20.5S32.714,9.5,30,9.5Z"
        fill="#ffffff"
    />

    <path d="M50,23.5H10a1.5,1.5,0,0,1,0-3H50a1.5,1.5,0,0,1,0,3Z" fill="#ffffff"/>

    <path
        d="M50,39.5H10a1.5,1.5,0,0,1,0-3H50a1.5,1.5,0,0,1,0,3Z" fill="#ffffff"/>
    <path
        d="M30,53.5A23.5,23.5,0,1,1,53.5,30,23.527,23.527,0,0,1,30,53.5Zm0-44A20.5,20.5,0,1,0,50.5,30,20.523,20.523,0,0,0,30,9.5Z"
        fill="#ffffff"
    />
  </svg>
</template>

<style scoped>

</style>