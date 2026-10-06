type Slot<T> = { current: T };
type Status = "saved" | "saving" | "error";

/** One writer, newest pending edit, optimistic revisions supplied by the owner.
 * Failed writes retain the newest edit for an explicit retry. Retired profile
 * queues neither send pending writes nor deliver stale acknowledgements. */
export class SaveCoordinator<T, R> {
  private retired = false;
  constructor(
    private pending: Slot<T | null>,
    private running: Slot<Promise<void> | null>,
    private commit: (value: T) => Promise<R>,
    private acknowledge: (result: R, submitted: T) => void,
    private status: (status: Status) => void,
    private ready: () => boolean = () => true,
  ) {}

  dispose() {
    this.retired = true;
    this.pending.current = null;
  }

  async flush(): Promise<void> {
    if (this.retired) return;
    if (this.running.current) {
      await this.running.current;
      return this.flush();
    }
    const operation = this.drain();
    this.running.current = operation;
    try {
      await operation;
    } finally {
      if (this.running.current === operation) this.running.current = null;
    }
    if (!this.retired && this.pending.current && this.ready())
      await this.flush();
  }

  private async drain() {
    while (!this.retired && this.pending.current && this.ready()) {
      const submitted = this.pending.current;
      this.pending.current = null;
      this.status("saving");
      try {
        const result = await this.commit(submitted);
        if (this.retired) return;
        this.acknowledge(result, submitted);
        this.status(this.pending.current ? "saving" : "saved");
      } catch (error) {
        if (this.retired) return;
        this.pending.current ??= submitted;
        this.status("error");
        throw error;
      }
    }
  }
}
