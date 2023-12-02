<script lang="ts" setup>
import { FTAudio, FTVideo, FTImage, FTText, FTMisc } from "@/lib/icons";
import { FileType } from "@/lib/api";
import { FunctionalComponent, SVGAttributes } from "vue";

const props = defineProps<{ filetype: FileType }>();

const ICON_MAP: Record<FileType, FunctionalComponent<SVGAttributes>> = {
  [FileType.UNKNOWN]: FTMisc,
  [FileType.AUDIO]: FTAudio,
  [FileType.VIDEO]: FTVideo,
  [FileType.DOCUMENT]: FTText,
  [FileType.IMAGE]: FTImage,
};
</script>

<template>
  <component :is="ICON_MAP[props.filetype]" />
</template>
