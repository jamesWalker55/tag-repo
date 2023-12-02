import { state } from "@/lib/api/state";
import { supportsAudioPlayback } from "@/lib/ffi";

export async function toggleAudioPreview() {
  if (state.audioPreview) {
    await disableAudioPreview();
  } else {
    await enableAudioPreview();
  }
}

export async function enableAudioPreview() {
  if (await supportsAudioPlayback()) {
    state.audioPreview = true;
  } else {
    alert(
      "Unable to initialize audio device.\nPlease ensure your audio devices are properly configured and restart the application.",
    );
  }
}

export async function disableAudioPreview() {
  state.audioPreview = false;
}

export async function setAudioVolume(volume: number) {
  // make sure volume is between 0 and 1
  volume = Math.max(0, Math.min(volume, 1));
  state.audioVolume = volume;
}
