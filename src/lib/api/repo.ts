import * as ffi from "@/lib/ffi";
import { open } from "@tauri-apps/api/dialog";
import { config } from ".";
import { state } from "./state";

export async function openRepo(path: string) {
  await ffi.openRepo(path);
  await config.setPath();
  await config.save()
}

export async function promptOpenRepo() {
  const path = await open({ directory: true, multiple: false });
  if (Array.isArray(path)) throw "cannot open multiple directories";

  if (path !== null) {
    await openRepo(path);
  }

  await config.setPath();
  await config.save()
}

export async function closeRepo() {
  await ffi.closeRepo();
  state.path = null;
  await config.setPath();
  await config.save()
}
