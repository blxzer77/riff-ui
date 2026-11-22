// Opens a reference page in headless Chrome, waits until it has reported its
// cells, then writes a full-page screenshot and the report.
//
//   node capture.mjs <page.html> <out.png> <scale> <width>
//
// Prints the report (JSON) on stdout. Driven over the DevTools protocol rather
// than `--screenshot --dump-dom`, which snapshot when a virtual-time budget
// runs out: a page still rendering animation frames never goes idle, so with
// many cells the snapshot raced the page and caught it half-mounted.
// Node's built-in WebSocket is the only dependency.

import { spawn } from "node:child_process";
import { writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [pagePath, outPath, scaleArg, widthArg] = process.argv.slice(2);
const scale = Number(scaleArg);
const width = Number(widthArg);
const TIMEOUT_MS = 60_000;

const profile = mkdtempSync(join(tmpdir(), "riff-reference-"));
const chrome = spawn(
  "google-chrome",
  [
    "--headless=new",
    "--disable-gpu",
    "--no-sandbox",
    "--hide-scrollbars",
    // The page and the fonts are file: URLs in different directories.
    "--allow-file-access-from-files",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "about:blank",
  ],
  { stdio: ["ignore", "ignore", "pipe"] },
);

const exited = new Promise((resolve) => chrome.once("exit", resolve));

/** Stops Chrome and removes its profile. Chrome writes to the profile while
 *  it shuts down, so removing it before the process has exited left the
 *  directory behind in /tmp on every run. */
async function cleanup() {
  if (chrome.exitCode === null && chrome.signalCode === null) chrome.kill();
  await Promise.race([exited, new Promise((r) => setTimeout(r, 5000))]);
  rmSync(profile, { recursive: true, force: true });
}

async function fail(message) {
  process.stderr.write(`capture: ${message}\n`);
  await cleanup();
  process.exit(1);
}

setTimeout(() => fail(`timed out after ${TIMEOUT_MS / 1000}s`), TIMEOUT_MS).unref();

// Chrome prints its DevTools endpoint on stderr.
const browserUrl = await new Promise((resolve) => {
  let buffer = "";
  chrome.stderr.on("data", (chunk) => {
    buffer += chunk;
    const match = buffer.match(/DevTools listening on (ws:\/\/\S+)/);
    if (match) resolve(match[1]);
  });
});

const port = new URL(browserUrl).port;
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const target = targets.find((t) => t.type === "page");
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.onopen = resolve;
  socket.onerror = reject;
});

let nextId = 1;
const pending = new Map();
socket.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result);
  }
};
function send(method, params = {}) {
  const id = nextId++;
  socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}
async function evaluate(expression) {
  const { result, exceptionDetails } = await send("Runtime.evaluate", {
    expression,
    returnByValue: true,
  });
  if (exceptionDetails) throw new Error(exceptionDetails.text);
  return result.value;
}

await send("Page.enable");
// Tall enough for any story, so the capture never resizes the viewport: a
// resize after the page settled would restart its CSS transitions and catch
// states (focus ring, press scale) at their first frame.
const VIEWPORT_HEIGHT = 4000;
await send("Emulation.setDeviceMetricsOverride", {
  width,
  height: VIEWPORT_HEIGHT,
  deviceScaleFactor: scale,
  mobile: false,
});
await send("Page.navigate", { url: pathToFileURL(pagePath).href });

// Wait for the page to report: a JSON object, or an error message.
let report = "";
while (!report) {
  await new Promise((r) => setTimeout(r, 100));
  report = await evaluate('document.getElementById("report")?.textContent ?? ""');
}
if (!report.startsWith("{")) fail(report.slice(0, 2000));

const { contentSize } = await send("Page.getLayoutMetrics");
const height = Math.ceil(contentSize.height);
if (height > VIEWPORT_HEIGHT) fail(`page is ${height}px tall; raise VIEWPORT_HEIGHT`);
const shot = await send("Page.captureScreenshot", {
  format: "png",
  clip: { x: 0, y: 0, width, height, scale: 1 },
});
writeFileSync(outPath, Buffer.from(shot.data, "base64"));
process.stdout.write(report);
await cleanup();
process.exit(0);
