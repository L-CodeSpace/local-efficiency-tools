import { basename, dirname, extension, joinPath, relativePath } from "./path";

export const imageExtensions = [
  "jpg",
  "jpeg",
  "png",
  "gif",
  "bmp",
  "tiff",
  "tif",
  "ico",
  "webp",
  "heic",
  "heif",
  "avif",
  "svg",
  "cr2",
  "nef",
  "arw",
];

export const videoExtensions = [
  "mp4",
  "mov",
  "mkv",
  "avi",
  "webm",
  "wmv",
  "m4v",
  "ts",
  "m2ts",
  "vob",
  "rmvb",
  "rm",
];

export function mediaKindForPath(path: string) {
  const ext = extension(path);
  if (imageExtensions.includes(ext)) return "image";
  if (videoExtensions.includes(ext)) return "video";
  return null;
}

export function plannedImageOutputPaths(
  inputs: string[],
  outputDir: string,
  format: "webp" | "avif",
  sourceRoot?: string,
) {
  return inputs.map((input) => outputPath(input, outputDir, `.${format}`, sourceRoot));
}

export function plannedVideoOutputPaths(
  inputs: string[],
  outputDir: string,
  targets: Array<"animatedWebp" | "av1WithAudio" | "av1VideoOnly" | "audioMp3">,
  sourceRoot?: string,
) {
  return inputs.flatMap((input) =>
    targets.map((target) => outputPath(input, outputDir, videoTargetSuffix(target), sourceRoot)),
  );
}

function videoTargetSuffix(target: "animatedWebp" | "av1WithAudio" | "av1VideoOnly" | "audioMp3") {
  if (target === "animatedWebp") return ".webp";
  if (target === "av1WithAudio") return ".av1.mp4";
  if (target === "av1VideoOnly") return ".av1-no-audio.mp4";
  return ".mp3";
}

function outputPath(input: string, outputDir: string, suffix: string, sourceRoot?: string) {
  const baseDir = outputDir.trim();
  if (sourceRoot && baseDir) {
    return joinPath(baseDir, replaceSuffix(relativePath(sourceRoot, input), suffix));
  }
  if (baseDir) {
    return joinPath(baseDir, replaceSuffix(basename(input), suffix));
  }
  const inputDir = dirname(input);
  const outputName = replaceSuffix(basename(input), suffix);
  return inputDir ? joinPath(inputDir, outputName) : outputName;
}

function replaceSuffix(path: string, suffix: string) {
  const dir = dirname(path);
  const name = basename(path);
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const nextName = `${stem}${suffix}`;
  return dir ? joinPath(dir, nextName) : nextName;
}
