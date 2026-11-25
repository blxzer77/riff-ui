// Maps a stories/tooltip.json cell to riff-uiux's <Tooltip>.

import { Tooltip } from "@riff/component";
import { Icon } from "@riff/gallery";

export const freezeCss = `
  .ui-tip { animation: none !important; }
`;

function stageStyle(side, align, stageWidth, stageHeight) {
  let justifyContent = "center";
  let alignItems = "center";
  let padding = "0px";

  if (side === "top") {
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
  } else if (side === "bottom") {
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
  side = "top",
  align = "center",
  offset = 6,
  content,
  shortcut,
  trigger = "button",
  triggerLabel = "Trigger",
  triggerIcon = "bookmark",
  triggerSize = "sm",
  stageWidth = 180,
  stageHeight = 84,
}) {
  const height = triggerSize === "md" ? 40 : 32;
  const iconSize = triggerSize === "md" ? 18 : 16;
  const baseStyle = {
    height,
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
    padding: 0,
  };

  const triggerEl =
    trigger === "icon-button" ? (
      <button
        type="button"
        aria-label={triggerLabel}
        style={{ ...baseStyle, width: height }}
      >
        <Icon name={triggerIcon} size={iconSize} />
      </button>
    ) : (
      <button type="button" style={{ ...baseStyle, paddingInline: 12 }}>
        {triggerLabel}
      </button>
    );

  return (
    <div style={stageStyle(side, align, stageWidth, stageHeight)}>
      <Tooltip open side={side} align={align} offset={offset} content={content} shortcut={shortcut}>
        {triggerEl}
      </Tooltip>
    </div>
  );
}
