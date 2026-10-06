export type CaptureTarget = "job" | "question";
export type Tab = { id?: number; url?: string; windowId?: number };
export type Sender = {
  id?: string;
  url?: string;
  tab?: Tab;
  frameId?: number;
  documentId?: string;
};
export type Port = {
  postMessage(message: unknown): void;
  disconnect(): void;
  onMessage: { addListener(listener: (message: unknown) => void): void };
  onDisconnect: { addListener(listener: () => void): void };
};
export interface ChromeApi {
  alarms: {
    create(name: string, info: { periodInMinutes: number }): Promise<void>;
    onAlarm: { addListener(listener: (alarm: { name: string }) => void): void };
  };
  runtime: {
    id: string;
    lastError?: { message?: string };
    connectNative(host: string): Port;
    sendMessage(message: unknown): Promise<unknown>;
    onMessage: {
      addListener(
        listener: (
          request: unknown,
          sender: Sender,
          reply: (response: unknown) => void,
        ) => boolean | void,
      ): void;
    };
    onStartup: { addListener(listener: () => void): void };
    onInstalled: { addListener(listener: () => void): void };
  };
  tabs: {
    query(query: {
      active: boolean;
      lastFocusedWindow: boolean;
    }): Promise<Tab[]>;
    get(id: number): Promise<Tab>;
    sendMessage(
      id: number,
      message: unknown,
      options: { documentId: string },
    ): Promise<unknown>;
    onActivated: {
      addListener(
        listener: (info: { tabId: number; windowId: number }) => void,
      ): void;
    };
    onRemoved: { addListener(listener: (id: number) => void): void };
    onUpdated: {
      addListener(
        listener: (
          id: number,
          change: { status?: string; url?: string },
        ) => void,
      ): void;
    };
  };
  windows: {
    onFocusChanged: { addListener(listener: (id: number) => void): void };
  };
  scripting: {
    executeScript(options: {
      target: { tabId: number; frameIds?: number[]; documentIds?: string[] };
      world: "ISOLATED";
      func: (...args: any[]) => unknown;
      args?: unknown[];
    }): Promise<{ frameId: number; documentId?: string; result?: unknown }[]>;
  };
}
declare global {
  const chrome: ChromeApi;
}
