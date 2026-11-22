// Renders an expanded story (see build.py) and reports each cell's box.
// build.py screenshots the page and crops those boxes into the pictures
// riff-story shows beside the live component.

import { createRoot } from "react-dom/client";

/** A Lucide icon, as riff-uiux's examples use them. A missing name leaves the
 *  slot empty, the way a broken image does. */
export function Icon({ name, size = "1em" }) {
  const svg = window.__RIFF_ICONS__[name];
  if (!svg) return null;
  return (
    <span
      aria-hidden="true"
      style={{ display: "inline-flex", width: size, height: size, flex: "none" }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

/** The pseudo-classes a pinned state implies. A pressed element is under
 *  the pointer too, so `active` also forces `hover`, as a real press would. */
const IMPLIES = { hover: ["hover"], active: ["hover", "active"], focus: ["focus"] };

/** A cell's wrapper, tagged with the state it pins. mount() marks the
 *  elements once the component has rendered (see pinStates). */
function Cell({ state, children }) {
  return (
    <div className="cell-inner" data-pin={state ?? undefined}>
      {children}
    </div>
  );
}

const PSEUDO = { hover: ":hover", active: ":active", focus: ":focus-visible" };

/** Copies every style rule that uses :hover, :active or :focus-visible with
 *  the pseudo-class swapped for an attribute, so `data-force-hover` shows
 *  exactly what hovering would. Generic: no component-specific selectors. */
function forceStates() {
  const extra = [];
  const found = new Set();
  const visit = (rules, wrap) => {
    for (const rule of rules) {
      if (rule.cssRules && rule.conditionText !== undefined) {
        // @media / @supports: keep the condition around the copies.
        const at = rule.cssText.slice(0, rule.cssText.indexOf("{")).trim();
        visit(rule.cssRules, (text) => wrap(`${at} { ${text} }`));
      } else if (rule.selectorText) {
        for (const [state, pseudo] of Object.entries(PSEUDO)) {
          if (!rule.selectorText.includes(pseudo)) continue;
          found.add(state);
          const selector = rule.selectorText.split(pseudo).join(`[data-force-${state}]`);
          const body = rule.cssText.slice(rule.cssText.indexOf("{"));
          extra.push(wrap(`${selector} ${body}`));
        }
      }
    }
  };
  for (const sheet of document.styleSheets) visit(sheet.cssRules, (text) => text);
  return { css: extra.join("\n"), found };
}

/** Marks every element of each pinned cell and installs the copied state
 *  rules. A component injects its stylesheet while rendering (React hoists
 *  it), so the rules may not exist on the first frame: wait for them, and
 *  fail rather than photograph a cell without the state it asks for. */
async function pinStates() {
  const pinned = [...document.querySelectorAll(".cell-inner[data-pin]")];
  if (!pinned.length) return; // no cell pins a state
  const wanted = new Set(pinned.flatMap((el) => IMPLIES[el.dataset.pin]));
  let result;
  for (let attempt = 0; attempt < 60; attempt++) {
    result = forceStates();
    if ([...wanted].every((state) => result.found.has(state))) break;
    await frame();
  }
  const missing = [...wanted].filter((state) => !result.found.has(state));
  if (missing.length) throw new Error(`no CSS rules for pinned states: ${missing.join(", ")}`);
  const style = document.createElement("style");
  style.textContent = result.css;
  document.head.append(style);
  for (const cell of pinned) {
    for (const el of [cell, ...cell.querySelectorAll("*")]) {
      for (const state of IMPLIES[cell.dataset.pin]) el.setAttribute(`data-force-${state}`, "");
    }
  }
}

export async function mount(story, cell) {
  if (story.freezeCss) {
    const style = document.createElement("style");
    style.textContent = story.freezeCss;
    document.head.append(style);
  }

  const boxes = [];
  const content = story.sections.map((section) => (
    <section key={section.id}>
      <h2>{section.title}</h2>
      {section.rows.map((row) => (
        <div className="row" key={row.id}>
          <span className="row-title">{row.title}</span>
          {row.cells.map((c) => (
            <div
              className="cell"
              key={c.key}
              ref={(el) => {
                if (el && !boxes.some((b) => b.key === c.key)) boxes.push({ key: c.key, el });
              }}
            >
              <Cell state={c.state}>{cell(c.props)}</Cell>
            </div>
          ))}
        </div>
      ))}
    </section>
  ));

  // Boxes must be measured with the bundled font, not whatever loads first.
  await document.fonts.ready;
  createRoot(document.getElementById("root")).render(content);
  // React 19 commits on its own schedule, not on the next frame: wait until
  // every cell is in the DOM before marking states or measuring.
  const total = story.sections.reduce((n, s) => n + s.rows.reduce((m, r) => m + r.cells.length, 0), 0);
  for (let attempt = 0; document.querySelectorAll(".cell-inner").length < total; attempt++) {
    if (attempt > 300) throw new Error(`only ${document.querySelectorAll(".cell-inner").length} of ${total} cells rendered`);
    await frame();
  }
  await pinStates();
  await frame();
  await frame();

  // A cell's box is its component's box; the picture is cropped with a margin
  // (build.py) so focus rings and shadows drawn outside it stay in view.
  return boxes.map(({ key, el }) => {
    const box = el.querySelector(".cell-inner").getBoundingClientRect();
    return { key, x: box.x + scrollX, y: box.y + scrollY, width: box.width, height: box.height };
  });
}

function frame() {
  return new Promise((r) => requestAnimationFrame(r));
}
