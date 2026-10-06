import { createCaptureController } from "./capture.js";
import { createNativeClient } from "./native-client.js";
import { createBridgeRuntime } from "./bridge-runtime.js";
const native = createNativeClient(chrome);
const controller = createCaptureController(chrome, native.request);
chrome.runtime.onMessage.addListener((request, sender, reply) => {
  void controller
    .handle(request, sender)
    .then(reply, () => reply({ alive: false }));
  return true;
});
const bridge = createBridgeRuntime(chrome, controller.poll, native.close);
void bridge.wake();
