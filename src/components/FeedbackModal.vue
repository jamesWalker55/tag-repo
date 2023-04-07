<script lang="ts" setup>
import { AppLogo } from "@/lib/icons";
interface Emits {
  (e: "closed"): void;
}

const emit = defineEmits<Emits>();
</script>

<template>
  <Teleport to="main">
    <div
      class="absolute bottom-0 left-0 right-0 top-0 bg-neutral-900 opacity-20"
      @click="emit('closed')"
    ></div>
    <div
      class="modal fixed left-1/2 top-1/2 z-0 flex max-h-full -translate-x-1/2 -translate-y-1/2 flex-row items-center justify-center gap-14 rounded bg-white px-14 py-10 shadow"
    >
      <div class="flex flex-col items-center">
        <AppLogo class="text-7xl text-orange-600" />
        <span class="text-xl font-bold">tagrepo</span>
        <span class="font-light">v2023.04.07</span>
      </div>
      <div class="h-full w-72 flex-1 overflow-x-auto py-2 text-sm">
        <p class="mb-2">
          Thanks for trying out
          <AppLogo class="inline text-sm text-orange-600" /> <b>tagrepo</b>!
        </p>
        <p class="mb-2">
          tagrepo is a software project that I am developing as part of my final
          year project for my Computer Science degree. As part of my evaluation
          process, I am seeking feedback from users like you.
        </p>
        <p class="mb-2">
          If you have any feedback about tagrepo, I would greatly appreciate it
          if you could take a moment to fill out the form linked below. Even if
          you are happy with the software and have no particular feedback, your
          response would still be valuable to me.
        </p>
        <p class="mb-4 mt-4 flex justify-center">
          <!-- TODO -->
          <a
            href="#TODO"
            class="rounded bg-violet-700 px-4 py-2 text-lg text-violet-50 shadow hover:bg-violet-600 hover:shadow-lg"
          >
            <span class="font-medium">Google</span> Forms
          </a>
        </p>
        <p class="mb-2">Thank you for your support!</p>
        <p class="mb-2 text-right italic">- James</p>
      </div>
    </div>
  </Teleport>
</template>
