#!/usr/bin/env python3
"""Renders riff-uiux reference pictures for riff-story's Compare tab.

A story is stories/<slug>.json, shared with riff-story: sections of rows of
cells, each cell plain-data props. For each stale story and each mode, this

  1. bundles the story with tools/reference/adapters/<slug>.jsx (which maps
     props to riff-uiux's component), riff-uiux's own tokens, React, the
     bundled Inter and the Lucide SVGs GPUI Kit ships;
  2. screenshots the page at 2x in headless Chrome, with cells marked
     `"state": "hover" | "active" | "focus"` pinned in that state;
  3. crops every cell into target/reference/<slug>/<mode>/<section>/<row>/<cell>.png
     and writes manifest.json: cell sizes, plus a hash of every input.

The hash covers the story, the adapter, this tool, the riff-uiux component and
tokens, and the fonts. riff-story compares the story hash with the story file
it was built with to show a reference as current or stale; `--check` reports
stories whose inputs changed.

The read-only riff-uiux checkout is only a module source: nothing from it is
copied into this repository, and the output is a build artifact under target/.

Usage:
  tools/reference/build.py                 # stale stories, both modes
  tools/reference/build.py button          # one story, even if current
  tools/reference/build.py --all           # every story
  tools/reference/build.py --check         # list stale stories, exit 1 if any
"""

import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

import PIL.Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOLS = ROOT / "tools/reference"
STORIES = ROOT / "stories"
OUT = ROOT / "target/reference"
BUILD = ROOT / "target/shots/reference"
FONTS = ROOT / "crates/riff-ui/assets/fonts"
FONT_FILES = ["Inter-Regular.ttf", "Inter-Medium.ttf"]

RIFF = pathlib.Path(os.environ.get("RIFF_UIUX_DIR", "/root/Projects/personal/affectui/riff-uiux"))
RIFF_COMPONENTS = RIFF / "dist/components"
RIFF_TOKENS = RIFF / "src/styles/tokens.css"
RIFF_NODE_MODULES = RIFF / "node_modules"
# The bundler riff-uiux's own gallery uses.
ESBUILD = RIFF / "examples/gallery/node_modules/.bin/esbuild"
# Lucide SVGs, as riff-uiux's examples use them and GPUI Kit ships them.
LUCIDE = pathlib.Path.home() / (
    ".cargo/registry/src/index.crates.io-1949cf8c6b5b557f/"
    "gpui-kit-assets-0.10.0/assets/icons"
)

# 2x for text and hairlines, as a HiDPI screen draws them.
SCALE = 2
PAGE_WIDTH = 1400
PAGE_BACKGROUND = {"light": "#ffffff", "dark": "#1c1c1e"}
# Pictures extend this far past the component's box on every side, for the
# focus ring (2px offset + 2px width) and drop shadows. riff-story draws the
# live component with the same margin, so the two stay aligned.
MARGIN = 8
MODES = ("light", "dark")

ENTRY = r"""
import { mount } from "./gallery.jsx";
import cell, * as adapter from "./adapter.jsx";
import story from "./story.json";

(async () => {
  const freezeCss = adapter.freezeCss?.replaceAll("@@FREEZE_MS@@", String(story.freeze_ms ?? 0));
  const cells = await mount({ ...story, freezeCss }, cell);
  document.getElementById("report").textContent = JSON.stringify({ cells });
})().catch(reportError);
"""

HTML = r"""<!doctype html>
<html lang="en" @@THEME@@>
<meta charset="utf-8">
<title>@@SLUG@@ — riff-uiux reference (@@MODE@@)</title>
<style>
  @@FONTFACE@@
  html, body { margin: 0; background: @@BACKGROUND@@; }
  body { padding: 24px; font-family: "Inter", ui-sans-serif, system-ui, sans-serif; }
  h2 { font: 12px ui-monospace, monospace; color: #737373; margin: 28px 0 10px; }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 28px; margin-bottom: 28px; }
  .row-title { width: 84px; font: 12px ui-monospace, monospace; color: #737373; flex: none; }
  .cell, .cell-inner { display: inline-flex; }
  #report { display: none; }
  /* Pictures show end states, not motion: with transitions on, a pinned
     state (riff-uiux eases colour and scale over 160ms) is caught wherever
     the transition happens to be when the screenshot is taken. Looping
     animations are held by each adapter's freezeCss instead. */
  *, *::before, *::after { transition: none !important; }
</style>
<style>
@@TOKENS@@
</style>
<body>
<div id="root"></div>
<pre id="report"></pre>
<script>
  // Any failure leaves its stack in the report, so this tool can quote it.
  function reportError(error) {
    document.getElementById("report").textContent = "error: " + ((error && error.stack) || error);
  }
  window.addEventListener("error", (event) => reportError(event.error ?? event.message));
  window.addEventListener("unhandledrejection", (event) => reportError(event.reason));
</script>
<script>@@ICONS@@</script>
<script>@@BUNDLE@@</script>
</body>
</html>
"""


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def fill(template, **values):
    """Substitutes `@@NAME@@` markers; str.format would trip on JS braces."""
    for name, value in values.items():
        template = template.replace(f"@@{name}@@", str(value))
    return template


# --- stories -----------------------------------------------------------------


def load_story(slug):
    """Reads a story and expands `"cells": "@<template>"` and row props, the
    same expansion riff-story's story module does."""
    path = STORIES / f"{slug}.json"
    if not path.exists():
        fail(f"no story {path.relative_to(ROOT)}")
    story = json.loads(path.read_text())
    templates = story.get("templates", {})
    for section in story["sections"]:
        for row in section["rows"]:
            cells = row["cells"]
            if isinstance(cells, str):
                name = cells.removeprefix("@")
                if name not in templates:
                    fail(f"{slug}: row {row['id']} names unknown template {cells!r}")
                cells = templates[name]
            row["cells"] = [
                {
                    "id": cell["id"],
                    "key": f"{section['id']}/{row['id']}/{cell['id']}",
                    "state": cell.get("state"),
                    "props": {**row.get("props", {}), **cell.get("props", {})},
                }
                for cell in cells
            ]
    keys = [c["key"] for s in story["sections"] for r in s["rows"] for c in r["cells"]]
    duplicates = {k for k in keys if keys.count(k) > 1}
    if duplicates:
        fail(f"{slug}: duplicate cells {sorted(duplicates)}")
    return story


def story_slugs():
    return sorted(path.stem for path in STORIES.glob("*.json"))


def input_files(slug):
    return [
        STORIES / f"{slug}.json",
        TOOLS / f"adapters/{slug}.jsx",
        TOOLS / "gallery.jsx",
        TOOLS / "capture.mjs",
        pathlib.Path(__file__),
        RIFF_COMPONENTS / f"{slug}.js",
        RIFF_TOKENS,
        *(FONTS / name for name in FONT_FILES),
    ]


def inputs_hash(slug):
    digest = hashlib.sha256()
    for path in input_files(slug):
        digest.update(path.name.encode())
        digest.update(path.read_bytes() if path.exists() else b"<missing>")
    return digest.hexdigest()[:16]


def story_hash(slug):
    """The story file alone: what riff-story can check without riff-uiux."""
    return hashlib.sha256((STORIES / f"{slug}.json").read_bytes()).hexdigest()[:16]


def is_current(slug):
    for mode in MODES:
        manifest = OUT / slug / mode / "manifest.json"
        if not manifest.exists():
            return False
        if json.loads(manifest.read_text()).get("inputs") != inputs_hash(slug):
            return False
    return True


# --- page --------------------------------------------------------------------


def font_faces():
    """@font-face rules for the Inter riff-ui embeds, named plainly "Inter" so
    riff-uiux's own `--riff-font: "Inter", …` stack picks it up."""
    rules = []
    for name in FONT_FILES:
        weight = 500 if "Medium" in name else 400
        url = (FONTS / name).resolve().as_uri()
        rules.append(
            f'@font-face {{ font-family: "Inter"; font-weight: {weight}; '
            f'src: url("{url}") format("truetype"); }}'
        )
    return "\n  ".join(rules)


def icons_script():
    if not LUCIDE.is_dir():
        fail(f"no Lucide icons at {LUCIDE}")
    icons = {
        path.stem: re.sub(r'\s(width|height)="24"', "", path.read_text())
        for path in sorted(LUCIDE.glob("*.svg"))
    }
    return f"window.__RIFF_ICONS__ = {json.dumps(icons)};"


def bundle(slug, story):
    if not ESBUILD.exists():
        fail(f"no esbuild at {ESBUILD} (riff-uiux's examples/gallery/node_modules)")
    adapter = TOOLS / f"adapters/{slug}.jsx"
    if not adapter.exists():
        fail(f"no adapter {adapter.relative_to(ROOT)}: it maps the story's props to riff-uiux")
    component = RIFF_COMPONENTS / f"{slug}.js"
    if not component.exists():
        fail(f"riff-uiux has no component {slug!r} ({component})")

    BUILD.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=BUILD) as tmp:
        tmp = pathlib.Path(tmp)
        (tmp / "entry.jsx").write_text(ENTRY)
        shutil.copy(TOOLS / "gallery.jsx", tmp / "gallery.jsx")
        shutil.copy(adapter, tmp / "adapter.jsx")
        (tmp / "story.json").write_text(json.dumps(story))
        # riff-uiux imports react by bare name; resolve it from the checkout.
        modules = tmp / "node_modules"
        modules.mkdir()
        for package in ("react", "react-dom", "scheduler"):
            (modules / package).symlink_to(RIFF_NODE_MODULES / package)
        out = tmp / "bundle.js"
        result = subprocess.run(
            [
                str(ESBUILD),
                str(tmp / "entry.jsx"),
                "--bundle",
                "--format=iife",
                f"--outfile={out}",
                "--jsx=automatic",
                '--define:process.env.NODE_ENV="production"',
                f"--alias:@riff/component={component}",
                f"--alias:@riff/gallery={tmp / 'gallery.jsx'}",
                "--log-level=warning",
            ],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            fail(f"esbuild failed for {slug}:\n{result.stderr}")
        return out.read_text()


def render(slug, mode, html):
    """Screenshots the page once it has reported its cells (capture.mjs)."""
    page = BUILD / f"{slug}-{mode}.html"
    page.write_text(html)
    png = BUILD / f"{slug}-{mode}.png"
    png.unlink(missing_ok=True)
    result = subprocess.run(
        ["node", str(TOOLS / "capture.mjs"), str(page), str(png), str(SCALE), str(PAGE_WIDTH)],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        fail(f"{slug}/{mode}: {result.stderr.strip()[-2000:]}")
    if not png.exists():
        fail(f"{slug}/{mode}: no screenshot written")
    return png, json.loads(result.stdout)


def crop(slug, mode, png, report, story):
    dest = OUT / slug / mode
    shutil.rmtree(dest, ignore_errors=True)
    dest.mkdir(parents=True)
    image = PIL.Image.open(png)
    cells = {}
    for box in report["cells"]:
        target = dest / f"{box['key']}.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        image.crop(
            (
                round((box["x"] - MARGIN) * SCALE),
                round((box["y"] - MARGIN) * SCALE),
                round((box["x"] + box["width"] + MARGIN) * SCALE),
                round((box["y"] + box["height"] + MARGIN) * SCALE),
            )
        ).save(target)
        cells[box["key"]] = {"width": round(box["width"], 2), "height": round(box["height"], 2)}
    expected = {c["key"] for s in story["sections"] for r in s["rows"] for c in r["cells"]}
    if expected != set(cells):
        fail(f"{slug}/{mode}: rendered cells differ from the story: {sorted(expected ^ set(cells))}")
    manifest = {
        "scale": SCALE,
        "margin": MARGIN,
        "inputs": inputs_hash(slug),
        "story": story_hash(slug),
        "cells": cells,
    }
    (dest / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")


def build(slug, icons, tokens, fontface):
    story = load_story(slug)
    code = bundle(slug, story)
    for mode in MODES:
        html = fill(
            HTML,
            SLUG=slug,
            MODE=mode,
            THEME='data-theme="dark"' if mode == "dark" else "",
            BACKGROUND=PAGE_BACKGROUND[mode],
            TOKENS=tokens,
            FONTFACE=fontface,
            ICONS=icons,
            BUNDLE=code,
        )
        png, report = render(slug, mode, html)
        crop(slug, mode, png, report, story)
    count = sum(len(r["cells"]) for s in story["sections"] for r in s["rows"])
    print(f"{slug}: {count} cells × {len(MODES)} modes → {(OUT / slug).relative_to(ROOT)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("slugs", nargs="*", help="stories to render, even if current")
    parser.add_argument("--all", action="store_true", help="render every story")
    parser.add_argument("--check", action="store_true", help="list stale stories and exit")
    args = parser.parse_args()

    if not RIFF_COMPONENTS.is_dir():
        fail(f"no riff-uiux checkout at {RIFF}")
    slugs = args.slugs or story_slugs()
    stale = [slug for slug in slugs if not is_current(slug)]

    if args.check:
        for slug in stale:
            print(f"stale: {slug}")
        sys.exit(1 if stale else 0)

    todo = slugs if (args.slugs or args.all) else stale
    if not todo:
        print("every reference is current")
        return
    icons, tokens, fontface = icons_script(), RIFF_TOKENS.read_text(), font_faces()
    for slug in todo:
        build(slug, icons, tokens, fontface)


if __name__ == "__main__":
    main()
