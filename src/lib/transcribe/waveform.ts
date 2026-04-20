export class WaveformRenderer {
  private raf: number | null = null;
  private analyser: AnalyserNode | null = null;

  constructor(private readonly canvas: HTMLCanvasElement) {}

  attach(analyser: AnalyserNode | null): void {
    this.detach();
    this.analyser = analyser;
    if (analyser) this.loop();
  }

  detach(): void {
    if (this.raf !== null) {
      cancelAnimationFrame(this.raf);
      this.raf = null;
    }
    this.analyser = null;
    this.clear();
  }

  private loop(): void {
    const analyser = this.analyser;
    if (!analyser) return;
    const ctx = this.canvas.getContext("2d");
    if (!ctx) return;
    const buffer = new Uint8Array(analyser.fftSize);

    const draw = () => {
      if (!this.analyser) return;
      analyser.getByteTimeDomainData(buffer);
      const { width, height } = this.canvas;
      ctx.clearRect(0, 0, width, height);
      const styles = getComputedStyle(this.canvas);
      const stroke = styles.getPropertyValue("--color-accent").trim() || "#3b82f6";
      ctx.strokeStyle = stroke;
      ctx.lineWidth = 2;
      ctx.beginPath();
      const slice = width / buffer.length;
      for (let i = 0; i < buffer.length; i += 1) {
        const v = buffer[i] / 128.0;
        const y = (v * height) / 2;
        const x = i * slice;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      this.raf = requestAnimationFrame(draw);
    };
    this.raf = requestAnimationFrame(draw);
  }

  private clear(): void {
    const ctx = this.canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }
}
