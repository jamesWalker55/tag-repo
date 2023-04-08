<script lang="ts" setup>
import {getItem, Item, openFile, revealFile, state} from '@/lib/api';
import {clipboard, path} from '@tauri-apps/api';
import { Ref, ref, shallowRef, watch } from "vue";
import {
  FTAudio,
  FTVideo,
  FTImage,
  FTText,
  FTFolder,
  FTMisc,
} from "@/lib/icons";

const props = defineProps<{ id: number }>();
const emit = defineEmits<{
  // select a single item with normal mouse click
  (e: "selection-set", id: number): void;
  // add to selection with ctrl + mouse click
  (e: "selection-add", id: number): void;
  // remove from selection with ctrl + mouse click
  (e: "selection-remove", id: number): void;
  // extend selection with shift + mouse click
  (e: "selection-extend", id: number): void;
}>();
const itemData: Ref<Item | null> = ref(null);
const itemIcon = shallowRef(FTMisc);

const EXT_MAP = new Map(
  Object.entries({
    aac: "audio",
    ac3: "audio",
    aif: "audio",
    aifc: "audio",
    aiff: "audio",
    au: "audio",
    cda: "audio",
    dts: "audio",
    fla: "audio",
    flac: "audio",
    it: "audio",
    m1a: "audio",
    m2a: "audio",
    m3u: "audio",
    m4a: "audio",
    mid: "audio",
    midi: "audio",
    mka: "audio",
    mod: "audio",
    mp2: "audio",
    mp3: "audio",
    mpa: "audio",
    ogg: "audio",
    ra: "audio",
    rmi: "audio",
    spc: "audio",
    snd: "audio",
    umx: "audio",
    voc: "audio",
    wav: "audio",
    wma: "audio",
    xm: "audio",
    opus: "audio",
    c: "document",
    chm: "document",
    cpp: "document",
    csv: "document",
    cxx: "document",
    doc: "document",
    docm: "document",
    docx: "document",
    dot: "document",
    dotm: "document",
    dotx: "document",
    h: "document",
    hpp: "document",
    htm: "document",
    html: "document",
    hxx: "document",
    ini: "document",
    java: "document",
    lua: "document",
    mht: "document",
    mhtml: "document",
    odt: "document",
    pdf: "document",
    potx: "document",
    potm: "document",
    ppam: "document",
    ppsm: "document",
    ppsx: "document",
    pps: "document",
    ppt: "document",
    pptm: "document",
    pptx: "document",
    rtf: "document",
    sldm: "document",
    sldx: "document",
    thmx: "document",
    txt: "document",
    vsd: "document",
    wpd: "document",
    wps: "document",
    wri: "document",
    xlam: "document",
    xls: "document",
    xlsb: "document",
    xlsm: "document",
    xlsx: "document",
    xltm: "document",
    xltx: "document",
    xml: "document",
    ani: "image",
    bmp: "image",
    gif: "image",
    ico: "image",
    jpe: "image",
    jpeg: "image",
    jpg: "image",
    pcx: "image",
    png: "image",
    psd: "image",
    tga: "image",
    tif: "image",
    tiff: "image",
    webp: "image",
    wmf: "image",
    "3g2": "video",
    "3gp": "video",
    "3gp2": "video",
    "3gpp": "video",
    amr: "video",
    amv: "video",
    asf: "video",
    avi: "video",
    bdmv: "video",
    bik: "video",
    d2v: "video",
    divx: "video",
    drc: "video",
    dsa: "video",
    dsm: "video",
    dss: "video",
    dsv: "video",
    evo: "video",
    f4v: "video",
    flc: "video",
    fli: "video",
    flic: "video",
    flv: "video",
    hdmov: "video",
    ifo: "video",
    ivf: "video",
    m1v: "video",
    m2p: "video",
    m2t: "video",
    m2ts: "video",
    m2v: "video",
    m4b: "video",
    m4p: "video",
    m4v: "video",
    mkv: "video",
    mp2v: "video",
    mp4: "video",
    mp4v: "video",
    mpe: "video",
    mpeg: "video",
    mpg: "video",
    mpls: "video",
    mpv2: "video",
    mpv4: "video",
    mov: "video",
    mts: "video",
    ogm: "video",
    ogv: "video",
    pss: "video",
    pva: "video",
    qt: "video",
    ram: "video",
    ratdvd: "video",
    rm: "video",
    rmm: "video",
    rmvb: "video",
    roq: "video",
    rpm: "video",
    smil: "video",
    smk: "video",
    swf: "video",
    tp: "video",
    tpr: "video",
    ts: "video",
    vob: "video",
    vp6: "video",
    webm: "video",
    wm: "video",
    wmp: "video",
    wmv: "video",
  })
);

async function getFileIcon(itemPath: string) {
  let fileType: string;
  try {
    const extension = (await path.extname(itemPath)).toLowerCase();
    fileType = EXT_MAP.get(extension) || "unknown";
  } catch (error) {
    fileType = "unknown";
  }
  if (fileType == "audio") {
    return FTAudio;
  } else if (fileType == "video") {
    return FTVideo;
  } else if (fileType == "document") {
    return FTText;
  } else if (fileType == "image") {
    return FTImage;
  } else {
    return FTMisc;
  }
}

async function fetchItemData(id: number) {
  const data = await getItem(id);

  const promises: Promise<void>[] = [
    (async () => {
      itemIcon.value = await getFileIcon(data.path);
    })(),
    (async () => {
      if (state.path === null) throw "Repo path is null!";
      itemFullPath.value = await path.join(state.path, data.path);
    })(),
  ];
  await Promise.allSettled(promises);

  itemData.value = data;
}

fetchItemData(props.id);

watch(
  () => props.id,
  async (newId) => {
    await fetchItemData(newId);
  }
);
</script>

<template>
  <div
    v-if="itemData !== null"
    class="item flex h-6 w-max items-center gap-1 truncate px-1"
    @click="
      async () => {
        if (state.path === null) throw 'repo path is null?!';
        if (itemData === null) throw 'item data is null?!';

        await clipboard.writeText(await path.join(state.path, itemData.path));
        // await revealFile(await path.join(state.path, itemData.path));
      }
    "
  >
    <itemIcon class="h-[16px] w-[16px] flex-none text-neutral-600" />
    <span class="text-sm">{{ itemData.path }}</span>
  </div>
  <div v-else>Loading...</div>
</template>

<style scoped>
.hover > .item {
  @apply bg-blue-50;
}
</style>
