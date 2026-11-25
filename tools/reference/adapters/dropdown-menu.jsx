// Maps a stories/dropdown-menu.json cell to riff-uiux's <DropdownMenu>.

import { DropdownMenu } from "@riff/component";
import { Icon } from "@riff/gallery";

export const freezeCss = `
  .ui-dm__panel, .ui-dm__tick { animation: none !important; stroke-dashoffset: 0 !important; }
`;

function stageStyle(side, align, stageWidth, stageHeight) {
  let justifyContent = align === "end" ? "flex-end" : "flex-start";
  let alignItems = side === "top" ? "flex-end" : "flex-start";
  let padding = "8px 10px";

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

function mapItems(items = []) {
  return items.map((item) => ({
    label: item.label,
    description: item.description,
    icon: item.icon ? <Icon name={item.icon} size="100%" /> : undefined,
    checked: item.checked,
    type: item.type,
    danger: item.danger,
    shortcut: item.shortcut,
    divider: item.divider,
    disabled: item.disabled,
    submenu: item.hasSubmenu ? [{ label: "Sub item" }] : undefined,
  }));
}

export default function cell({
  side = "bottom",
  align = "start",
  title,
  headerTitle,
  headerMeta,
  width,
  empty = "Nothing here yet",
  items = [],
  triggerLabel = "Actions",
  stageWidth = 220,
  stageHeight = 190,
}) {
  const triggerStyle = {
    height: 32,
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
    whiteSpace: "nowrap",
  };

  const header =
    headerTitle || headerMeta ? (
      <div
        style={{
          padding: "8px 10px 6px",
          borderBottom: "1px solid var(--riff-border)",
          marginBottom: 4,
          display: "flex",
          flexDirection: "column",
          gap: 1,
        }}
      >
        {headerTitle && (
          <strong style={{ fontSize: 13, lineHeight: "18px", fontWeight: 500, color: "var(--riff-ink)" }}>
            {headerTitle}
          </strong>
        )}
        {headerMeta && (
          <span style={{ fontSize: 12, lineHeight: "16px", color: "var(--riff-muted)" }}>
            {headerMeta}
          </span>
        )}
      </div>
    ) : undefined;

  return (
    <div style={stageStyle(side, align, stageWidth, stageHeight)}>
      <DropdownMenu
        side={side}
        align={align}
        title={title}
        header={header}
        width={width}
        empty={empty}
        open={true}
        items={mapItems(items)}
        trigger={({ triggerProps }) => (
          <button type="button" {...triggerProps} style={triggerStyle}>
            {triggerLabel}
          </button>
        )}
      />
    </div>
  );
}
