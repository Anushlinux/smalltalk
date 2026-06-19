import { browser } from "wxt/browser";
import { chunkTextBlocks, normalizeWhitespace, scoreTextMatch } from "../src/shared/chunking";
import type {
  AppType,
  AttentionEvent,
  CapturedMessage,
  ExtensionMessage,
  ExtensionResponse,
  PageSnapshot,
  ResumeTarget
} from "../src/shared/types";

let captureEnabled = false;
let lastCursorSent = 0;
let lastScrollSent = 0;
let lastSelection = "";

export default defineContentScript({
  matches: ["http://*/*", "https://*/*"],
  runAt: "document_idle",
  main() {
    injectStyles();
    browser.runtime.onMessage.addListener((rawMessage: unknown): Promise<ExtensionResponse> | undefined => {
      const message = rawMessage as ExtensionMessage;
      if (message.type === "CAPTURE_PAGE_SNAPSHOT") {
        captureEnabled = true;
        return Promise.resolve({ ok: true, snapshot: capturePageSnapshot() });
      }
      if (message.type === "SET_CAPTURE_ENABLED") {
        captureEnabled = message.enabled;
        if (!captureEnabled) lastSelection = "";
        return Promise.resolve({ ok: true });
      }
      if (message.type === "APPLY_RESUME_HIGHLIGHT") {
        const applied = applyResumeHighlight(message.target);
        return Promise.resolve(applied ? { ok: true } : { ok: false, error: "Could not find target on this page." });
      }
      return undefined;
    });

    browser.runtime
      .sendMessage({ type: "GET_SESSION_STATE" } satisfies ExtensionMessage)
      .then((rawResponse) => {
        const response = rawResponse as ExtensionResponse;
        captureEnabled = Boolean(response.ok && response.state?.activeSession);
      })
      .catch(() => {
        captureEnabled = false;
      });

    window.addEventListener("mousemove", handleMouseMove, { passive: true });
    window.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("mouseup", handleSelection, { passive: true });
    window.addEventListener("keyup", handleSelection, { passive: true });
    document.addEventListener("click", handleClick, true);
    document.addEventListener("visibilitychange", handleVisibility);
    window.addEventListener("pagehide", handlePageHide, { passive: true });
  }
});

function capturePageSnapshot(): PageSnapshot {
  const blocks = collectTextBlocks();
  const appType = inferAppType(location.href);
  const selectedText = normalizeWhitespace(document.getSelection()?.toString() ?? "");
  const centerElement = document.elementFromPoint(Math.round(window.innerWidth / 2), Math.round(window.innerHeight * 0.45));
  const centerReadable = nearestReadableElement(centerElement);

  return {
    url: location.href,
    title: document.title,
    appType,
    visibleText: blocks.map((block) => block.text).join("\n").slice(0, 4200),
    activeMessage: captureActiveMessage(appType),
    selectedText: selectedText.length >= 18 ? selectedText.slice(0, 800) : undefined,
    centerText: centerReadable ? normalizeWhitespace(centerReadable.innerText || centerReadable.textContent || "").slice(0, 900) : undefined,
    chunks: chunkTextBlocks(blocks).map((chunk) => ({
      ...chunk,
      title: document.title,
      url: location.href
    })),
    capturedAt: Date.now(),
    scrollY: window.scrollY
  };
}

function collectTextBlocks() {
  const root = document.querySelector("article") ?? document.querySelector("main") ?? document.body;
  const elements = Array.from(root.querySelectorAll<HTMLElement>("h1,h2,h3,h4,p,li,blockquote,section"));
  const blocks: Array<{ text: string; heading?: string; selector?: string; scrollY?: number }> = [];
  let currentHeading = findNearestHeading(root as HTMLElement)?.textContent ?? document.title;

  for (const element of elements) {
    if (!isVisible(element) || isChromeLike(element)) continue;
    const tag = element.tagName.toLowerCase();
    const text = normalizeWhitespace(element.innerText || element.textContent || "");
    if (!text) continue;
    if (/^h[1-4]$/.test(tag)) {
      currentHeading = text;
      continue;
    }
    element.dataset.smalltalkAnchor = element.dataset.smalltalkAnchor || `st-${blocks.length}-${Date.now()}`;
    blocks.push({
      text,
      heading: currentHeading,
      selector: selectorFor(element),
      scrollY: element.getBoundingClientRect().top + window.scrollY
    });
  }

  return blocks;
}

function nearestReadableElement(target: EventTarget | null): HTMLElement | undefined {
  if (!(target instanceof Element)) return undefined;
  const element = target.closest<HTMLElement>("p,li,blockquote,section,article,main");
  if (!element || !isVisible(element) || isChromeLike(element)) return undefined;
  const text = normalizeWhitespace(element.innerText || element.textContent || "");
  return text.length > 20 ? element : undefined;
}

function handleMouseMove(event: MouseEvent) {
  if (!captureEnabled || Date.now() - lastCursorSent < 1400) return;
  const element = nearestReadableElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!element) return;
  lastCursorSent = Date.now();
  sendAttention({
    kind: "cursor_dwell",
    url: location.href,
    title: document.title,
    timestamp: Date.now(),
    textQuote: normalizeWhitespace(element.innerText || element.textContent || "").slice(0, 240),
    scrollY: window.scrollY,
    viewportHeight: window.innerHeight,
    x: event.clientX,
    y: event.clientY
  });
}

function handleScroll() {
  if (!captureEnabled || Date.now() - lastScrollSent < 1200) return;
  lastScrollSent = Date.now();
  const centerElement = document.elementFromPoint(Math.round(window.innerWidth / 2), Math.round(window.innerHeight * 0.45));
  const readable = nearestReadableElement(centerElement);
  const maxScroll = Math.max(1, document.documentElement.scrollHeight - window.innerHeight);
  sendAttention({
    kind: "scroll",
    url: location.href,
    title: document.title,
    timestamp: Date.now(),
    textQuote: readable ? normalizeWhitespace(readable.innerText || readable.textContent || "").slice(0, 240) : undefined,
    scrollY: window.scrollY,
    viewportHeight: window.innerHeight,
    value: Math.min(1, window.scrollY / maxScroll)
  });
}

function handleSelection() {
  if (!captureEnabled) return;
  const selection = document.getSelection();
  const selectedText = normalizeWhitespace(selection?.toString() ?? "");
  if (selectedText.length < 18 || selectedText === lastSelection) return;
  lastSelection = selectedText;
  const container = selection?.anchorNode?.parentElement;
  const readable = nearestReadableElement(container ?? null);
  sendAttention({
    kind: "selection",
    url: location.href,
    title: document.title,
    timestamp: Date.now(),
    textQuote: readable ? normalizeWhitespace(readable.innerText || readable.textContent || "").slice(0, 240) : selectedText.slice(0, 240),
    selectedText: selectedText.slice(0, 600),
    scrollY: window.scrollY,
    viewportHeight: window.innerHeight
  });
}

function handleClick(event: MouseEvent) {
  if (!captureEnabled) return;
  const link = (event.target as Element | null)?.closest<HTMLAnchorElement>("a[href]");
  if (!link?.href) return;
  const readable = nearestReadableElement(link) ?? nearestReadableElement(event.target);
  browser.runtime
    .sendMessage({
      type: "RECORD_LINK_CLICK",
      event: {
        kind: "link_click",
        url: location.href,
        title: document.title,
        timestamp: Date.now(),
        targetHref: link.href,
        targetText: normalizeWhitespace(link.innerText || link.textContent || "").slice(0, 160),
        textQuote: readable ? normalizeWhitespace(readable.innerText || readable.textContent || "").slice(0, 240) : undefined,
        scrollY: window.scrollY,
        viewportHeight: window.innerHeight
      }
    } satisfies ExtensionMessage)
    .catch(() => undefined);
}

function handleVisibility() {
  if (!captureEnabled) return;
  sendAttention({
    kind: "visibility",
    url: location.href,
    title: document.title,
    timestamp: Date.now(),
    value: document.visibilityState === "visible" ? 1 : 0,
    scrollY: window.scrollY,
    viewportHeight: window.innerHeight
  });

  if (document.visibilityState === "hidden") sendSnapshot();
}

function handlePageHide() {
  if (!captureEnabled) return;
  sendSnapshot();
}

function sendAttention(event: Omit<AttentionEvent, "id" | "sessionId" | "visitId" | "tabId">) {
  browser.runtime
    .sendMessage({ type: "RECORD_ATTENTION_EVENT", event } satisfies ExtensionMessage)
    .catch(() => undefined);
}

function sendSnapshot() {
  browser.runtime
    .sendMessage({ type: "PAGE_SNAPSHOT_CAPTURED", snapshot: capturePageSnapshot() } satisfies ExtensionMessage)
    .catch(() => undefined);
}

function applyResumeHighlight(target: ResumeTarget): boolean {
  removeExistingHighlight();
  const element = findTargetElement(target);
  if (!element) {
    if (typeof target.scrollY === "number") window.scrollTo({ top: target.scrollY, behavior: "smooth" });
    showResumeOverlay(target);
    return false;
  }

  element.classList.add("smalltalk-resume-target");
  element.scrollIntoView({ block: "center", behavior: "smooth" });
  showResumeOverlay(target);
  return true;
}

function findTargetElement(target: ResumeTarget): HTMLElement | undefined {
  if (target.selector) {
    const selected = document.querySelector<HTMLElement>(target.selector);
    if (selected && scoreTextMatch(selected.innerText || selected.textContent || "", target.textQuote) > 0.25) return selected;
  }

  const readable = Array.from(document.querySelectorAll<HTMLElement>("p,li,blockquote,section"));
  const byQuote = readable
    .map((element) => ({
      element,
      score: scoreTextMatch(element.innerText || element.textContent || "", target.textQuote)
    }))
    .sort((a, b) => b.score - a.score)[0];
  if (byQuote?.score >= 0.38) return byQuote.element;

  if (target.heading) {
    const heading = Array.from(document.querySelectorAll<HTMLElement>("h1,h2,h3,h4")).find((item) =>
      normalizeWhitespace(item.innerText || item.textContent || "").toLowerCase().includes(target.heading!.toLowerCase().slice(0, 40))
    );
    const next = heading?.nextElementSibling;
    if (next instanceof HTMLElement) return next;
  }

  return undefined;
}

function showResumeOverlay(target: ResumeTarget) {
  const existing = document.getElementById("smalltalk-resume-overlay");
  existing?.remove();

  const overlay = document.createElement("div");
  overlay.id = "smalltalk-resume-overlay";
  overlay.innerHTML = `
    <button aria-label="Dismiss">x</button>
    <strong>Continue here</strong>
    <span>${escapeHtml(target.reason)}</span>
  `;
  overlay.querySelector("button")?.addEventListener("click", () => overlay.remove());
  document.documentElement.appendChild(overlay);
}

function removeExistingHighlight() {
  document.querySelectorAll(".smalltalk-resume-target").forEach((element) => element.classList.remove("smalltalk-resume-target"));
}

function injectStyles() {
  if (document.getElementById("smalltalk-resume-style")) return;
  const style = document.createElement("style");
  style.id = "smalltalk-resume-style";
  style.textContent = `
    .smalltalk-resume-target {
      outline: 3px solid #24c6a5 !important;
      background: linear-gradient(90deg, rgba(36,198,165,.18), rgba(255,214,102,.2)) !important;
      border-radius: 6px !important;
      box-shadow: 0 0 0 8px rgba(36,198,165,.08) !important;
    }
    #smalltalk-resume-overlay {
      position: fixed;
      right: 18px;
      bottom: 18px;
      z-index: 2147483647;
      width: min(340px, calc(100vw - 36px));
      padding: 14px 16px;
      color: #101314;
      background: #fbfffd;
      border: 1px solid rgba(16,19,20,.16);
      border-left: 4px solid #24c6a5;
      border-radius: 8px;
      box-shadow: 0 18px 48px rgba(16,19,20,.22);
      font: 500 13px/1.4 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    #smalltalk-resume-overlay strong,
    #smalltalk-resume-overlay span {
      display: block;
      padding-right: 24px;
    }
    #smalltalk-resume-overlay span {
      margin-top: 4px;
      color: #3f4747;
      font-weight: 450;
    }
    #smalltalk-resume-overlay button {
      position: absolute;
      top: 8px;
      right: 8px;
      width: 24px;
      height: 24px;
      border: 0;
      border-radius: 99px;
      background: transparent;
      color: #596161;
      cursor: pointer;
    }
  `;
  document.documentElement.appendChild(style);
}

function selectorFor(element: HTMLElement): string {
  if (element.dataset.smalltalkAnchor) return `[data-smalltalk-anchor="${element.dataset.smalltalkAnchor}"]`;
  const parts: string[] = [];
  let current: Element | null = element;
  while (current && current !== document.body && parts.length < 5) {
    const tag = current.tagName.toLowerCase();
    const parent: Element | null = current.parentElement;
    if (!parent) break;
    const currentTag = current.tagName;
    const siblings = Array.from(parent.children).filter((child: Element) => child.tagName === currentTag);
    const index = siblings.indexOf(current) + 1;
    parts.unshift(`${tag}:nth-of-type(${Math.max(1, index)})`);
    current = parent;
  }
  return parts.length ? parts.join(" > ") : "body";
}

function findNearestHeading(root: HTMLElement): HTMLElement | undefined {
  return root.querySelector<HTMLElement>("h1,h2,h3,h4") ?? undefined;
}

function captureActiveMessage(appType: AppType): CapturedMessage | undefined {
  if (appType === "chatgpt") {
    const messages = Array.from(
      document.querySelectorAll<HTMLElement>('[data-message-author-role], article, [data-testid^="conversation-turn"]')
    )
      .filter((element) => isVisible(element) && !isChromeLike(element))
      .map((element) => ({
        element,
        distance: distanceFromViewportCenter(element),
        text: normalizeWhitespace(element.innerText || element.textContent || "")
      }))
      .filter((item) => item.text.length >= 20)
      .sort((a, b) => a.distance - b.distance);
    const nearest = messages[0];
    if (!nearest) return undefined;
    const role = nearest.element.getAttribute("data-message-author-role");
    return {
      role: role === "user" || role === "assistant" ? role : "unknown",
      text: nearest.text.slice(0, 1800),
      selector: selectorFor(nearest.element)
    };
  }

  const centerElement = document.elementFromPoint(Math.round(window.innerWidth / 2), Math.round(window.innerHeight * 0.45));
  const readable = nearestReadableElement(centerElement);
  if (!readable) return undefined;
  return {
    role: "unknown",
    text: normalizeWhitespace(readable.innerText || readable.textContent || "").slice(0, 1200),
    selector: selectorFor(readable)
  };
}

function inferAppType(url: string): AppType {
  try {
    const host = new URL(url).hostname;
    if (host.includes("chatgpt.com")) return "chatgpt";
    if (host.includes("github.com")) return "github";
    if (host.includes("notion.so")) return "notion";
    if (host.includes("mail.google.com")) return "email";
    if (host.includes("docs.google.com") || host.includes("developer.chrome.com")) return "docs";
  } catch {
    return "other";
  }
  return "other";
}

function distanceFromViewportCenter(element: HTMLElement): number {
  const rect = element.getBoundingClientRect();
  const elementCenter = rect.top + rect.height / 2;
  return Math.abs(elementCenter - window.innerHeight * 0.45);
}

function isVisible(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  const style = window.getComputedStyle(element);
  return rect.width > 0 && rect.height > 0 && style.visibility !== "hidden" && style.display !== "none";
}

function isChromeLike(element: HTMLElement): boolean {
  return Boolean(element.closest("nav,header,footer,aside,form,button,[role='navigation'],[aria-hidden='true']"));
}

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (char) => {
    const entities: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#039;"
    };
    return entities[char] ?? char;
  });
}
