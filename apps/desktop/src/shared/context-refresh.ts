/** Coalesces events behind one request and pauses the heartbeat while hidden. */
export class ContextRefresh {
  private active = true;
  private visible = false;
  private running = false;
  private queued = false;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(
    private readonly fetch: () => Promise<void>,
    private readonly interval = 10_000,
  ) {}

  setVisible(visible: boolean) {
    this.visible = visible;
    this.clearTimer();
    if (visible) this.refresh();
    else this.queued = false;
  }

  refresh = () => {
    if (!this.active || !this.visible) return;
    this.clearTimer();
    if (this.running) {
      this.queued = true;
      return;
    }
    this.running = true;
    void this.fetch().finally(() => {
      this.running = false;
      if (!this.active || !this.visible) return;
      if (this.queued) {
        this.queued = false;
        this.refresh();
      } else this.timer = setTimeout(this.refresh, this.interval);
    });
  };

  stop() {
    this.active = false;
    this.queued = false;
    this.clearTimer();
  }
  private clearTimer() {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = undefined;
  }
}
