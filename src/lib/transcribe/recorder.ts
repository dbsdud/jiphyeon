import { buildFilename, pickSupportedFormat, type RecordingFormat } from "./format";

export interface RecorderCallbacks {
  onChunkSaved: (filename: string, blob: Blob) => Promise<void> | void;
  onError: (message: string) => void;
  onElapsed: (ms: number) => void;
  onAnalyser: (analyser: AnalyserNode | null) => void;
}

const MAX_DURATION_MS = 60 * 60 * 1000;

export class RollingRecorder {
  private stream: MediaStream | null = null;
  private recorder: MediaRecorder | null = null;
  private audioContext: AudioContext | null = null;
  private analyser: AnalyserNode | null = null;
  private format: RecordingFormat | null = null;
  private sequence = 0;
  private startedAt = 0;
  private elapsedTimer: ReturnType<typeof setInterval> | null = null;
  private rolling = false;

  constructor(private readonly callbacks: RecorderCallbacks) {}

  get isRecording(): boolean {
    return this.recorder?.state === "recording";
  }

  async start(): Promise<void> {
    const format = pickSupportedFormat();
    if (!format) {
      this.callbacks.onError("이 환경에서는 녹음 포맷을 지원하지 않습니다.");
      return;
    }
    this.format = format;
    try {
      this.stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch (err) {
      this.callbacks.onError(`마이크 권한을 얻지 못했습니다: ${String(err)}`);
      return;
    }
    this.setupAnalyser(this.stream);
    this.sequence = 0;
    this.startedAt = Date.now();
    this.elapsedTimer = setInterval(() => {
      this.callbacks.onElapsed(Date.now() - this.startedAt);
    }, 250);
    this.startChunk();
  }

  stop(): void {
    this.rolling = false;
    if (this.recorder && this.recorder.state !== "inactive") {
      this.recorder.stop();
    }
    if (this.elapsedTimer) {
      clearInterval(this.elapsedTimer);
      this.elapsedTimer = null;
    }
  }

  private setupAnalyser(stream: MediaStream): void {
    const Ctx = window.AudioContext ?? (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctx) return;
    this.audioContext = new Ctx();
    const source = this.audioContext.createMediaStreamSource(stream);
    this.analyser = this.audioContext.createAnalyser();
    this.analyser.fftSize = 1024;
    source.connect(this.analyser);
    this.callbacks.onAnalyser(this.analyser);
  }

  private teardown(): void {
    this.stream?.getTracks().forEach((t) => t.stop());
    this.stream = null;
    this.audioContext?.close().catch(() => {});
    this.audioContext = null;
    this.analyser = null;
    this.callbacks.onAnalyser(null);
  }

  private startChunk(): void {
    if (!this.stream || !this.format) return;
    this.sequence += 1;
    const format = this.format;
    const filename = buildFilename(new Date(), this.sequence, format.extension);
    const recorder = new MediaRecorder(this.stream, { mimeType: format.mimeType });
    const chunks: Blob[] = [];
    recorder.ondataavailable = (e) => {
      if (e.data && e.data.size > 0) chunks.push(e.data);
    };
    recorder.onstop = async () => {
      const blob = new Blob(chunks, { type: format.mimeType });
      if (blob.size > 0) {
        try {
          await this.callbacks.onChunkSaved(filename, blob);
        } catch (err) {
          this.callbacks.onError(`저장 실패: ${String(err)}`);
        }
      }
      if (this.rolling) {
        this.startChunk();
      } else {
        this.teardown();
      }
    };
    recorder.onerror = () => {
      this.callbacks.onError("녹음 중 오류가 발생했습니다.");
      this.rolling = false;
    };
    recorder.start();
    this.recorder = recorder;
    this.rolling = true;
    setTimeout(() => {
      if (this.rolling && this.recorder === recorder && recorder.state === "recording") {
        recorder.stop();
      }
    }, MAX_DURATION_MS);
  }
}
