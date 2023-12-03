<script lang="ts" setup>
import { config } from "@/lib/api";
import { createEventListenerRegistry } from "@/lib/utils";
import { ref, watch } from "vue";

interface Props {
  initialValue?: number;
  min?: number;
  max?: number;
  disabled?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  initialValue: 1,
  min: 0,
  max: 1,
  disabled: false,
});

interface Emits {
  (e: "valueChanged", value: number): void;
}

const emit = defineEmits<Emits>();

const knobValue = ref(props.initialValue);
watch(
  () => knobValue.value,
  (newValue) => emit("valueChanged", newValue),
);

const CHANGE_RATE = 0.005;

function onKnobMouseDown(downEvt: MouseEvent) {
  // don't move knob if widget is disabled
  if (props.disabled) return;

  const listeners = createEventListenerRegistry();
  const initialX = downEvt.clientX;
  const initialValue = knobValue.value;
  function updateKnobValue(mouseEvt: MouseEvent) {
    const dx = mouseEvt.clientX - initialX;
    let newValue = initialValue + dx * CHANGE_RATE;
    newValue = Math.max(props.min, Math.min(newValue, props.max));
    knobValue.value = newValue;
  }
  listeners.add(window, "mousemove", (moveEvt: MouseEvent) => {
    updateKnobValue(moveEvt);
  });
  listeners.add(window, "mouseup", (upEvt: MouseEvent) => {
    updateKnobValue(upEvt);
    listeners.clear();
    config.setAudioPreview().then(config.save);
  });
}
</script>

<template>
  <div class="flex flex-col">
    <div class="mx-2 mt-1" :class="!disabled ? '' : 'text-neutral-400'">
      Volume
    </div>
    <div class="mx-2 mb-2 grid flex-1 grid-cols-1 grid-rows-1">
      <!-- bg layer -->
      <div
        class="col-start-1 row-start-1 mx-2 flex flex-col items-center justify-center"
      >
        <div
          ref="sliderLine"
          class="h-0.5 w-full"
          :class="!disabled ? 'bg-neutral-300' : 'bg-neutral-200'"
        ></div>
      </div>
      <!-- foregound layer -->
      <div
        class="col-start-1 row-start-1 flex flex-row items-center justify-center"
      >
        <div :style="{ flex: knobValue - min }"></div>
        <div
          ref="sliderKnob"
          class="h-4 w-4 rounded-full border"
          :class="
            !disabled
              ? 'border-neutral-400 bg-neutral-50 drop-shadow hover:bg-white active:border-slate-400 active:bg-slate-200'
              : ''
          "
          @mousedown="onKnobMouseDown"
        ></div>
        <div :style="{ flex: max - knobValue }"></div>
      </div>
    </div>
  </div>
</template>

<style scoped></style>
