export interface RecordingFormat {
  mimeType: string;
  extension: string;
}

const FORMAT_CANDIDATES: readonly RecordingFormat[] = [
  { mimeType: "audio/mp4;codecs=mp4a.40.2", extension: "m4a" },
  { mimeType: "audio/webm;codecs=opus", extension: "webm" },
  { mimeType: "audio/ogg;codecs=opus", extension: "ogg" },
] as const;

const EXTENSION_BY_MIME: Record<string, string> = {
  "audio/mp4": "m4a",
  "audio/x-m4a": "m4a",
  "audio/webm": "webm",
  "audio/ogg": "ogg",
};

export function pickSupportedFormat(): RecordingFormat | null {
  if (
    typeof MediaRecorder === "undefined" ||
    typeof MediaRecorder.isTypeSupported !== "function"
  ) {
    return null;
  }
  for (const candidate of FORMAT_CANDIDATES) {
    if (MediaRecorder.isTypeSupported(candidate.mimeType)) {
      return { ...candidate };
    }
  }
  return null;
}

function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

export function buildFilename(now: Date, sequence: number, ext: string): string {
  const y = now.getFullYear();
  const m = pad2(now.getMonth() + 1);
  const d = pad2(now.getDate());
  const hh = pad2(now.getHours());
  const mm = pad2(now.getMinutes());
  const seq = pad2(sequence);
  const cleanExt = ext.startsWith(".") ? ext.slice(1) : ext;
  return `${y}-${m}-${d}-${hh}${mm}-${seq}.${cleanExt}`;
}

/** MIME 타입에서 파일 확장자 추정. 매칭 실패 시 fallback. */
export function extensionForMime(mime: string, fallback = "webm"): string {
  const normalized = mime.split(";")[0].trim().toLowerCase();
  return EXTENSION_BY_MIME[normalized] ?? fallback;
}
