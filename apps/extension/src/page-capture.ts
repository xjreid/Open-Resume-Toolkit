// This function is serialized into Chrome's isolated world. Keep helpers inside
// it: no imported code, page-world globals, external messages, or form values.
export function startPageCapture(
  sessionId: string,
  expiresAt: number,
): unknown {
  const ownWindow = window as typeof window & {
    __ortCapture?: { id: string; stop: () => void };
  };
  if (ownWindow.__ortCapture?.id === sessionId) return { ready: true };
  ownWindow.__ortCapture?.stop();
  if (location.href.length > 16 * 1024 || !document.documentElement)
    return { error: "PAGE_UNAVAILABLE" };
  if (typeof Highlight === "undefined" || !CSS.highlights)
    return { error: "PAGE_UNAVAILABLE" };
  const url = location.href;
  // Anchors are content coordinates, not viewport coordinates. A selected
  // scroll pane has its own origin; otherwise use the document's scroll origin.
  let scroller: Element | null = null;
  let anchor: { x: number; y: number } | null = null;
  let cursor = { x: 0, y: 0 };
  let initialOffset = { x: 0, y: 0 };
  let hasScrolled = false;
  let stopped = false;
  const highlightName = `ort-capture-${sessionId.replace(/[^a-z0-9]/g, "")}`;
  const highlight = new Highlight();
  highlight.priority = 1;
  const highlightStyle = new CSSStyleSheet();
  highlightStyle.replaceSync(
    `::highlight(${highlightName}) { background-color: #93c5fd; color: #111827; }`,
  );
  const styledRoots = new Set<Document | ShadowRoot>();
  let previewTimer: ReturnType<typeof setTimeout> | undefined;
  let previewFrame = 0;
  let lastPreview = 0;
  let completedRect: {
    left: number;
    top: number;
    right: number;
    bottom: number;
  } | null = null;
  const shield = document.createElement("div");
  shield.setAttribute("data-ort-capture", "shield");
  // A closed shadow root isolates capture rendering from the site's styles.
  const shadow = shield.attachShadow({ mode: "closed" });
  const box = document.createElement("div");
  box.style.cssText =
    "position:absolute;box-sizing:border-box;border:2px solid #2563eb;background:rgba(37,99,235,.12);pointer-events:none;display:none;";
  const boxViewport = document.createElement("div");
  boxViewport.style.cssText =
    "position:absolute;inset:0;overflow:hidden;pointer-events:none;";
  boxViewport.append(box);
  shadow.append(boxViewport);
  shield.style.cssText =
    "all:initial!important;position:fixed!important;inset:0!important;z-index:2147483647!important;display:block!important;cursor:crosshair!important;background:transparent!important;";
  document.documentElement.append(shield);
  function send(phase: string, extra: Record<string, unknown> = {}) {
    return Promise.resolve().then(() =>
      chrome.runtime.sendMessage({
        kind: "capture.page",
        sessionId,
        phase,
        ...extra,
      }),
    );
  }
  function cleanup() {
    if (stopped) return;
    stopped = true;
    shield.remove();
    clearTimeout(previewTimer);
    cancelAnimationFrame(previewFrame);
    CSS.highlights.delete(highlightName);
    for (const root of styledRoots)
      root.adoptedStyleSheets = root.adoptedStyleSheets.filter(
        (sheet) => sheet !== highlightStyle,
      );
    styledRoots.clear();
    window.removeEventListener("pointerdown", click, true);
    window.removeEventListener("click", swallow, true);
    window.removeEventListener("pointermove", move, true);
    window.removeEventListener("wheel", wheel, true);
    window.removeEventListener("keydown", key, true);
    window.removeEventListener("scroll", scroll, true);
    window.removeEventListener("resize", changed);
    window.removeEventListener("pagehide", changed);
    clearInterval(heartbeat);
    clearTimeout(expiry);
    if (ownWindow.__ortCapture?.id === sessionId) delete ownWindow.__ortCapture;
  }
  function fail(code: string) {
    cleanup();
    void send("failed", { code }).catch(() => {});
  }
  function changed() {
    fail("PAGE_CHANGED");
  }
  function parentOf(element: Element): Element | null {
    return (
      element.parentElement ??
      (element.getRootNode() as ShadowRoot).host ??
      null
    );
  }
  function beneath(x: number, y: number) {
    shield.style.setProperty("pointer-events", "none", "important");
    try {
      let hit = document.elementFromPoint(x, y);
      while (hit?.shadowRoot) {
        const inner = hit.shadowRoot.elementFromPoint(x, y);
        if (!inner || inner === hit) break;
        hit = inner;
      }
      return hit;
    } finally {
      shield.style.removeProperty("pointer-events");
    }
  }
  function scrollable(element: Element, axis: "x" | "y") {
    const css = getComputedStyle(element);
    return (
      /^(auto|scroll|overlay)$/.test(
        axis === "x" ? css.overflowX : css.overflowY,
      ) &&
      (axis === "x"
        ? element.scrollWidth > element.clientWidth
        : element.scrollHeight > element.clientHeight)
    );
  }
  function origin() {
    if (!scroller) return { x: scrollX, y: scrollY };
    if (!scroller.isConnected) throw new Error("PAGE_CHANGED");
    const bounds = scroller.getBoundingClientRect();
    return {
      x: scroller.scrollLeft - bounds.left - scroller.clientLeft,
      y: scroller.scrollTop - bounds.top - scroller.clientTop,
    };
  }
  function draw() {
    if (!anchor || stopped || completedRect) return;
    const offset = origin();
    if (offset.x !== initialOffset.x || offset.y !== initialOffset.y)
      hasScrolled = true;
    const start = { x: anchor.x - offset.x, y: anchor.y - offset.y };
    const bounds = scroller?.getBoundingClientRect();
    const left = bounds ? bounds.left + scroller!.clientLeft : 0;
    const top = bounds ? bounds.top + scroller!.clientTop : 0;
    boxViewport.style.left = `${left}px`;
    boxViewport.style.top = `${top}px`;
    boxViewport.style.width = `${scroller?.clientWidth ?? innerWidth}px`;
    boxViewport.style.height = `${scroller?.clientHeight ?? innerHeight}px`;
    box.style.left = `${Math.min(start.x, cursor.x) - left}px`;
    box.style.top = `${Math.min(start.y, cursor.y) - top}px`;
    box.style.width = `${Math.abs(cursor.x - start.x)}px`;
    box.style.height = `${Math.abs(cursor.y - start.y)}px`;
    schedulePreview();
  }
  function schedulePreview() {
    if (previewTimer !== undefined || previewFrame || stopped || completedRect)
      return;
    // Coalesce movement/scroll events. The box tracks every move; text geometry
    // is refreshed at most twenty times per second instead of blocking scrolling.
    previewTimer = setTimeout(
      () => {
        previewTimer = undefined;
        previewFrame = requestAnimationFrame(() => {
          previewFrame = 0;
          lastPreview = performance.now();
          if (!anchor || stopped || completedRect) return;
          shield.style.setProperty("pointer-events", "none", "important");
          try {
            const offset = origin();
            const first = { x: anchor.x - offset.x, y: anchor.y - offset.y };
            const selected = extract(
              {
                left: Math.min(first.x, cursor.x),
                top: Math.min(first.y, cursor.y),
                right: Math.max(first.x, cursor.x),
                bottom: Math.max(first.y, cursor.y),
              },
              true,
            );
            highlight.clear();
            for (const range of selected.ranges) {
              const root = range.startContainer.getRootNode();
              if (
                (root instanceof Document || root instanceof ShadowRoot) &&
                !styledRoots.has(root)
              ) {
                root.adoptedStyleSheets = [
                  ...root.adoptedStyleSheets,
                  highlightStyle,
                ];
                styledRoots.add(root);
              }
              highlight.add(range);
            }
            CSS.highlights.set(highlightName, highlight);
          } catch {
            // Never leave an old region highlighted after a move that cannot be
            // previewed within the bounded geometry budget. Final intake remains
            // independently bounded and never transmits a partial extraction.
            highlight.clear();
          } finally {
            shield.style.removeProperty("pointer-events");
          }
        });
      },
      Math.max(0, 50 - (performance.now() - lastPreview)),
    );
  }
  function scroll() {
    try {
      draw();
    } catch {
      changed();
    }
  }
  function wheel(event: WheelEvent) {
    if (!event.isTrusted || stopped || event.ctrlKey || event.metaKey) return;
    // The shield protects page controls, so route wheel scrolling to the real
    // element beneath it, including embedded job-description scroll panes.
    event.preventDefault();
    event.stopImmediatePropagation();
    cursor = { x: event.clientX, y: event.clientY };
    const unit =
      event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? innerHeight : 1;
    let dx = event.deltaX * unit,
      dy = event.deltaY * unit;
    for (
      let element = beneath(event.clientX, event.clientY);
      element &&
      element !== document.body &&
      element !== document.documentElement;
      element = parentOf(element)
    ) {
      const x = element.scrollLeft,
        y = element.scrollTop;
      element.scrollBy({
        left: scrollable(element, "x") ? dx : 0,
        top: scrollable(element, "y") ? dy : 0,
        behavior: "instant",
      });
      dx -= element.scrollLeft - x;
      dy -= element.scrollTop - y;
      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) break;
    }
    if (dx || dy) window.scrollBy({ left: dx, top: dy, behavior: "instant" });
    scroll();
  }
  function swallow(event: Event) {
    if (!stopped) {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.type === "click" && event.isTrusted && completedRect)
        finish(completedRect);
    }
  }
  function move(event: PointerEvent) {
    cursor = { x: event.clientX, y: event.clientY };
    scroll();
  }
  function key(event: KeyboardEvent) {
    if (event.isTrusted && event.key === "Escape") {
      swallow(event);
      cleanup();
      void send("cancelled").catch(() => {});
    }
  }
  function extract(
    rect: {
      left: number;
      top: number;
      right: number;
      bottom: number;
    },
    preview = false,
  ) {
    const start = performance.now();
    const budget = preview ? 120 : 1_000;
    const chunks: string[] = [];
    const ranges: Range[] = [];
    let length = 0,
      visited = 0;
    let previousBlock: Element | null = null;
    const range = document.createRange();
    const styleCache = new Map<Element, CSSStyleDeclaration>();
    function style(element: Element) {
      let value = styleCache.get(element);
      if (!value) {
        value = getComputedStyle(element);
        styleCache.set(element, value);
      }
      return value;
    }
    function parent(element: Element): Element | null {
      return parentOf(element);
    }
    function info(element: Element) {
      let block: Element | null = null;
      let clip = {
        left: -Infinity,
        top: -Infinity,
        right: Infinity,
        bottom: Infinity,
      };
      let withinContent = true;
      for (
        let ancestor: Element | null = element;
        ancestor;
        ancestor = parent(ancestor)
      ) {
        if (
          ancestor.matches(
            "script,style,noscript,template,input,textarea,select,[contenteditable],[hidden],[aria-hidden='true']",
          )
        )
          return null;
        const css = style(ancestor);
        if (
          css.display === "none" ||
          css.visibility !== "visible" ||
          Number(css.opacity) === 0
        )
          return null;
        // Fixed/sticky UI is not part of text spanning scrolled document content.
        // A fixed ancestor containing the entire selected pane is its viewport.
        if (
          hasScrolled &&
          withinContent &&
          ancestor !== scroller &&
          ["fixed", "sticky"].includes(css.position)
        )
          return null;
        if (!block && !["inline", "contents"].includes(css.display))
          block = ancestor;
        if (ancestor === scroller) withinContent = false;
        if (
          withinContent &&
          ancestor !== document.body &&
          ancestor !== document.documentElement &&
          (css.overflowX !== "visible" || css.overflowY !== "visible")
        ) {
          const bounds = ancestor.getBoundingClientRect();
          if (css.overflowX !== "visible") {
            clip.left = Math.max(clip.left, bounds.left);
            clip.right = Math.min(clip.right, bounds.right);
          }
          if (css.overflowY !== "visible") {
            clip.top = Math.max(clip.top, bounds.top);
            clip.bottom = Math.min(clip.bottom, bounds.bottom);
          }
        }
      }
      return {
        block,
        clip,
        pre:
          style(element).whiteSpace.startsWith("pre") ||
          style(element).whiteSpace === "break-spaces",
      };
    }
    function intersects(
      bounds: DOMRect,
      clip: { left: number; right: number; top: number; bottom: number },
    ) {
      return (
        bounds.width > 0 &&
        bounds.height > 0 &&
        bounds.right > Math.max(rect.left, clip.left) &&
        bounds.left < Math.min(rect.right, clip.right) &&
        bounds.bottom > Math.max(rect.top, clip.top) &&
        bounds.top < Math.min(rect.bottom, clip.bottom)
      );
    }
    function visible(
      bounds: DOMRect,
      element: Element,
      clip: { left: number; right: number; top: number; bottom: number },
    ) {
      const x = (bounds.left + bounds.right) / 2,
        y = (bounds.top + bounds.bottom) / 2;
      if (
        x < rect.left ||
        x > rect.right ||
        y < rect.top ||
        y > rect.bottom ||
        x < clip.left ||
        x > clip.right ||
        y < clip.top ||
        y > clip.bottom
      )
        return false;
      // Off-screen layout still belongs to the explicitly selected content box.
      // Hit testing only works in the viewport; CSS/overflow clipping is checked
      // above for all text, including the first corner scrolled out of view.
      if (x < 0 || y < 0 || x >= innerWidth || y >= innerHeight) return true;
      const hit = document.elementFromPoint(x, y);
      if (!hit) return false;
      if (hit === element || element.contains(hit) || hit.contains(element))
        return true;
      for (
        let ancestor = parent(element);
        ancestor;
        ancestor = parent(ancestor)
      )
        if (ancestor === hit) return true;
      return false;
    }
    function add(text: string, block: Element | null) {
      if (!text) return;
      if (chunks.length && previousBlock !== block) {
        chunks.push("\n");
        length++;
      }
      chunks.push(text);
      length += text.length;
      previousBlock = block;
      if (length > 128 * 1024) throw new Error("CAPTURE_TOO_LARGE");
    }
    function include(text: Text, start: number, end: number) {
      if (!preview || end <= start) return;
      if (ranges.length >= 8_192) throw new Error("CAPTURE_TOO_LARGE");
      const selected = document.createRange();
      selected.setStart(text, start);
      selected.setEnd(text, end);
      ranges.push(selected);
    }
    // Walk DOM reading order, including accessible open shadow trees. Closed
    // frames/shadow roots and image/canvas text are intentionally not OCR'd.
    function walk(node: Node) {
      if (++visited > 30_000 || performance.now() - start > budget)
        throw new Error("CAPTURE_TOO_LARGE");
      if (node.nodeType === Node.TEXT_NODE) {
        const text = node as Text,
          element = text.parentElement;
        if (!element || !text.data.length) return;
        const metadata = info(element);
        if (!metadata) return;
        range.selectNodeContents(text);
        const bounds = [...range.getClientRects()];
        if (!bounds.some((bounds) => intersects(bounds, metadata.clip))) return;
        // Preview paints only currently visible text. Final extraction also
        // includes selected text above/below the viewport after scrolling.
        if (
          preview &&
          !bounds.some(
            (bounds) =>
              bounds.bottom > 0 &&
              bounds.top < innerHeight &&
              bounds.right > 0 &&
              bounds.left < innerWidth,
          )
        )
          return;
        // Whole-node fast path when every rendered line is selected and visible.
        if (
          bounds.length &&
          bounds.every(
            (bounds) =>
              bounds.left >= rect.left &&
              bounds.right <= rect.right &&
              bounds.top >= rect.top &&
              bounds.bottom <= rect.bottom &&
              bounds.left >= metadata.clip.left &&
              bounds.right <= metadata.clip.right &&
              bounds.top >= metadata.clip.top &&
              bounds.bottom <= metadata.clip.bottom &&
              visible(bounds, element, metadata.clip),
          )
        ) {
          add(
            metadata.pre ? text.data : text.data.replace(/\s+/g, " "),
            metadata.block,
          );
          include(text, 0, text.data.length);
          return;
        }
        let result = "";
        let run: number | null = null;
        for (let offset = 0; offset < text.data.length; ) {
          if (offset % 128 === 0 && performance.now() - start > budget)
            throw new Error("CAPTURE_TOO_LARGE");
          const point = text.data.codePointAt(offset)!;
          const size = point > 0xffff ? 2 : 1;
          range.setStart(text, offset);
          range.setEnd(text, offset + size);
          const glyph = range.getBoundingClientRect();
          if (
            intersects(glyph, metadata.clip) &&
            visible(glyph, element, metadata.clip)
          ) {
            if (run === null) run = offset;
            result += text.data.slice(offset, offset + size);
          } else if (run !== null) {
            include(text, run, offset);
            run = null;
          }
          offset += size;
          if (result.length + length > 128 * 1024)
            throw new Error("CAPTURE_TOO_LARGE");
        }
        if (run !== null) include(text, run, text.data.length);
        add(
          metadata.pre ? result : result.replace(/\s+/g, " "),
          metadata.block,
        );
        return;
      }
      if (
        node instanceof Element &&
        node.matches(
          "script,style,noscript,template,input,textarea,select,[contenteditable],[hidden],[aria-hidden='true']",
        )
      )
        return;
      if (node instanceof HTMLBRElement) {
        const metadata = info(node);
        const bounds = node.getBoundingClientRect();
        const y = (bounds.top + bounds.bottom) / 2;
        if (
          metadata &&
          bounds.height > 0 &&
          bounds.left >= Math.max(rect.left, metadata.clip.left) &&
          bounds.left <= Math.min(rect.right, metadata.clip.right) &&
          y >= Math.max(rect.top, metadata.clip.top) &&
          y <= Math.min(rect.bottom, metadata.clip.bottom)
        )
          add("\n", metadata.block);
        return;
      }
      if (node instanceof HTMLSlotElement) {
        const assigned = node.assignedNodes({ flatten: true });
        for (const child of assigned.length ? assigned : node.childNodes)
          walk(child);
        return;
      }
      const children =
        node instanceof Element && node.shadowRoot
          ? node.shadowRoot.childNodes
          : node.childNodes;
      for (const child of children) walk(child);
    }
    walk(scroller ?? document.body ?? document.documentElement);
    const text = chunks
      .join("")
      .normalize("NFC")
      .replace(/\r\n?/g, "\n")
      .trim();
    if (!text && !preview) throw new Error("EMPTY_SELECTION");
    if (new TextEncoder().encode(text).length > 128 * 1024)
      throw new Error("CAPTURE_TOO_LARGE");
    return { text, ranges };
  }
  function click(event: PointerEvent) {
    if (!event.isTrusted || event.button !== 0 || stopped) return;
    swallow(event);
    if (!anchor) {
      for (
        let element = beneath(event.clientX, event.clientY);
        element &&
        element !== document.body &&
        element !== document.documentElement;
        element = parentOf(element)
      ) {
        if (scrollable(element, "x") || scrollable(element, "y")) {
          scroller = element;
          break;
        }
      }
      initialOffset = origin();
      anchor = {
        x: event.clientX + initialOffset.x,
        y: event.clientY + initialOffset.y,
      };
      box.style.display = "block";
      move(event);
      void send("selecting")
        .then((response) => {
          if (!(response as { alive?: boolean })?.alive && !stopped) cleanup();
        })
        .catch(cleanup);
      return;
    }
    const offset = origin();
    const end = { x: event.clientX + offset.x, y: event.clientY + offset.y };
    if (offset.x !== initialOffset.x || offset.y !== initialOffset.y)
      hasScrolled = true;
    completedRect = {
      left: Math.min(anchor.x, end.x),
      top: Math.min(anchor.y, end.y),
      right: Math.max(anchor.x, end.x),
      bottom: Math.max(anchor.y, end.y),
    };
  }
  function finish(rect: {
    left: number;
    top: number;
    right: number;
    bottom: number;
  }) {
    cleanup();
    try {
      if (rect.right - rect.left < 2 || rect.bottom - rect.top < 2)
        throw new Error("EMPTY_SELECTION");
      if (location.href !== url) throw new Error("PAGE_CHANGED");
      const offset = origin();
      const { text } = extract({
        left: rect.left - offset.x,
        top: rect.top - offset.y,
        right: rect.right - offset.x,
        bottom: rect.bottom - offset.y,
      });
      const title = new TextDecoder().decode(
        new TextEncoder().encode(document.title.slice(0, 500)),
      );
      void send("completed", { text, url, title }).catch(() => {});
    } catch (error) {
      void send("failed", {
        code: error instanceof Error ? error.message : "CAPTURE_INVALID",
      }).catch(() => {});
    }
  }
  // One listener per document, reused across generations; never trust page events.
  const listenerWindow = ownWindow as typeof ownWindow & {
    __ortCaptureListener?: boolean;
  };
  if (!listenerWindow.__ortCaptureListener) {
    chrome.runtime.onMessage.addListener((request: unknown, sender, reply) => {
      const value = request as { kind?: string; sessionId?: string };
      if (
        sender.id === chrome.runtime.id &&
        !sender.tab &&
        value.kind === "capture.cancel" &&
        ownWindow.__ortCapture?.id === value.sessionId
      ) {
        ownWindow.__ortCapture!.stop();
        reply({ ok: true });
      }
    });
    listenerWindow.__ortCaptureListener = true;
  }
  const expiry = setTimeout(
    () => fail("CAPTURE_EXPIRED"),
    Math.max(0, expiresAt - Date.now()),
  );
  let probing = false;
  const heartbeat = setInterval(() => {
    if (probing || stopped) return;
    probing = true;
    let timeout: ReturnType<typeof setTimeout>;
    void Promise.race([
      Promise.resolve().then(() =>
        chrome.runtime.sendMessage({ kind: "capture.ping", sessionId }),
      ),
      new Promise((_, reject) => {
        timeout = setTimeout(() => reject(new Error()), 3_000);
      }),
    ])
      .then((response) => {
        if (!(response as { alive?: boolean })?.alive) cleanup();
      })
      .catch(cleanup)
      .finally(() => {
        clearTimeout(timeout);
        probing = false;
      });
  }, 750);
  ownWindow.__ortCapture = { id: sessionId, stop: cleanup };
  window.addEventListener("pointerdown", click, true);
  window.addEventListener("click", swallow, true);
  window.addEventListener("pointermove", move, true);
  window.addEventListener("wheel", wheel, { capture: true, passive: false });
  window.addEventListener("keydown", key, true);
  window.addEventListener("scroll", scroll, true);
  window.addEventListener("resize", changed);
  window.addEventListener("pagehide", changed);
  return { ready: true };
}
