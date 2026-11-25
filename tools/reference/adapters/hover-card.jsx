// Maps a stories/hover-card.json cell to riff-uiux's <HoverCard>.

import { HoverCard } from "@riff/component";

export const freezeCss = `
  .ui-hvc { animation: none !important; }
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
  offset = 8,
  width,
  title,
  handle,
  meta,
  linkLabel,
  triggerLabel = "@alice",
  triggerHeight = 28,
  stageWidth = 220,
  stageHeight = 136,
}) {
  const triggerStyle = {
    height: triggerHeight,
    paddingInline: 10,
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
    textDecoration: "none",
  };

  const content = (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {(title || handle) && (
        <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
          {title && <strong style={{ fontWeight: 600, color: "var(--riff-ink)" }}>{title}</strong>}
          {handle && (
            <span style={{ fontSize: 12, lineHeight: "16px", color: "var(--riff-muted)" }}>
              {handle}
            </span>
          )}
        </div>
      )}
      {meta && (
        <div style={{ margin: 0, color: "var(--riff-muted)" }}>
          {meta}
        </div>
      )}
      {linkLabel && (
        <span style={{ marginTop: 4, fontSize: 12.5, lineHeight: "18px", fontWeight: 500, color: "var(--riff-info)" }}>
          {linkLabel}
        </span>
      )}
    </div>
  );

  return (
    <div style={stageStyle(side, align, stageWidth, stageHeight)}>
      <HoverCard
        side={side}
        align={align}
        offset={offset}
        width={width}
        open={true}
        content={content}
      >
        <a href="#preview" style={triggerStyle}>
          {triggerLabel}
        </a>
      </HoverCard>
    </div>
  );
}
