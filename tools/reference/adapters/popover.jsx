// Maps a stories/popover.json cell to riff-uiux's <Popover>.

import { Popover } from "@riff/component";

export const freezeCss = `
  .ui-pop { animation: none !important; }
`;

function stageStyle(side, align, stageWidth, stageHeight) {
  let justifyContent = "center";
  let alignItems = "center";
  let padding = "0px";

  if (side === "bottom") {
    alignItems = "flex-start";
    if (align === "start") {
      justifyContent = "flex-start";
      padding = "8px 0 0 10px";
    } else if (align === "end") {
      justifyContent = "flex-end";
      padding = "8px 10px 0 0";
    } else {
      padding = "8px 0 0 0";
    }
  } else if (side === "top") {
    alignItems = "flex-end";
    if (align === "start") {
      justifyContent = "flex-start";
      padding = "0 0 8px 10px";
    } else if (align === "end") {
      justifyContent = "flex-end";
      padding = "0 10px 8px 0";
    } else {
      padding = "0 0 8px 0";
    }
  } else if (side === "left") {
    justifyContent = "flex-end";
    if (align === "start") {
      alignItems = "flex-start";
      padding = "8px 8px 0 0";
    } else if (align === "end") {
      alignItems = "flex-end";
      padding = "0 8px 8px 0";
    } else {
      padding = "0 8px 0 0";
    }
  } else if (side === "right") {
    justifyContent = "flex-start";
    if (align === "start") {
      alignItems = "flex-start";
      padding = "8px 0 0 8px";
    } else if (align === "end") {
      alignItems = "flex-end";
      padding = "0 0 8px 8px";
    } else {
      padding = "0 0 0 8px";
    }
  }

  return {
    width: stageWidth,
    height: stageHeight,
    display: "flex",
    justifyContent,
    alignItems,
    padding,
    boxSizing: "border-box",
  };
}

export default function cell({
  side = "bottom",
  align = "start",
  offset = 6,
  title,
  description,
  body,
  width,
  unstyled = false,
  triggerLabel = "Open",
  triggerHeight = 32,
  stageWidth = 240,
  stageHeight = 140,
}) {
  const triggerStyle = {
    height: triggerHeight,
    paddingInline: 12,
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    borderRadius: "var(--riff-radius, 7px)",
    border: "1px solid var(--riff-border)",
    background: "var(--riff-surface)",
    color: "var(--riff-ink)",
    boxSizing: "border-box",
    fontFamily: "var(--riff-font)",
    fontSize: "13px",
    lineHeight: "18px",
    fontWeight: 500,
    paddingTop: 0,
    paddingBottom: 0,
  };

  const unstyledStyle = unstyled
    ? {
        padding: "8px 12px",
        borderRadius: "8px",
        border: "1px dashed var(--riff-border-strong)",
        background: "var(--riff-subtle)",
        color: "var(--riff-ink)",
        fontFamily: "var(--riff-font)",
        fontSize: "12.5px",
        lineHeight: "18px",
      }
    : undefined;

  return (
    <div style={stageStyle(side, align, stageWidth, stageHeight)}>
      <span style={{ display: "inline-flex" }}>
        <button type="button" style={triggerStyle}>
          {triggerLabel}
        </button>
        <Popover
          open
          onClose={() => {}}
          side={side}
          align={align}
          offset={offset}
          title={title}
          description={description}
          width={width}
          unstyled={unstyled}
          style={unstyledStyle}
        >
          {body ? <div>{body}</div> : undefined}
        </Popover>
      </span>
    </div>
  );
}
