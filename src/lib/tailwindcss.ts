import resolveConfig from "tailwindcss/resolveConfig";
import rawConfig from "../../tailwind.config";
import { parseRemSize } from "@/lib/utils";

const config = resolveConfig(rawConfig);

export default config;

export function getFontSize(size: string) {
  const remSize = config.theme!.fontSize![size];
  if (typeof remSize !== "string") {
    throw `invalid rem size definition: ${remSize}`;
  }
  const pxSize = parseRemSize(remSize);
  if (pxSize === null) {
    throw `invalid rem size definition: ${remSize}`;
  }
  return pxSize;
}

export function getSpacingSize(size: string) {
  const remSize = config.theme!.spacing![size];
  const pxSize = parseRemSize(remSize);
  if (pxSize === null) {
    throw `invalid rem size definition: ${remSize}`;
  }
  return pxSize;
}
