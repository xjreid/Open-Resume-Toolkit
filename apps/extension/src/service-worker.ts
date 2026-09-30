import { createCaptureController } from "./capture.js";
import { createNativeClient } from "./native-client.js";
const native = createNativeClient(chrome);
const controller = createCaptureController(chrome, native.request);
chrome.runtime.onMessage.addListener((request, sender, reply) => {
  void controller.handle(request, sender).then(reply);
  return true;
});
const poll = () => void controller.poll();
chrome.runtime.onStartup.addListener(poll);
chrome.runtime.onInstalled.addListener(poll);
setInterval(poll, 500);
poll();
