// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { ProfileBoundary } from "./ProfileBoundary";
const native = vi.hoisted(() => ({
  replace: undefined as (() => void) | undefined,
  stop: vi.fn(),
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: async (_event: string, callback: () => void) => {
      native.replace = callback;
      return native.stop;
    },
  }),
}));
function Owner() {
  const [value, setValue] = useState("fresh");
  return <button onClick={() => setValue("old profile")}>{value}</button>;
}
it("remounts profile owners and retires its native listener", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () =>
      root.render(
        <ProfileBoundary>
          <Owner />
        </ProfileBoundary>,
      ),
    );
    await act(async () => host.querySelector("button")!.click());
    expect(host.textContent).toBe("old profile");
    await act(async () => native.replace!());
    expect(host.textContent).toBe("fresh");
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
  expect(native.stop).toHaveBeenCalledTimes(1);
});
