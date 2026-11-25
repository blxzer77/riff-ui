// Maps a stories/context-menu.json cell to riff-uiux's <ContextMenu>.

import { useEffect, useRef } from "react";
import { ContextMenu } from "@riff/component";
import { Icon } from "@riff/gallery";

export const freezeCss = `
  .ui-ctx__panel, .ui-ctx__tick { animation: none !important; stroke-dashoffset: 0 !important; }
`;

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
    submenu: item.hasSubmenu ? [{ label: "Inbox" }] : undefined,
  }));
}

function ContextMenuCell({
  title,
  width,
  offsetX = 24,
  offsetY = 24,
  items = [],
  stageWidth = 230,
  stageHeight = 210,
}) {
  const areaRef = useRef(null);

  useEffect(() => {
    const el = areaRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    el.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: rect.left + offsetX,
        clientY: rect.top + offsetY,
      }),
    );
  }, [offsetX, offsetY]);

  const areaStyle = {
    width: stageWidth,
    height: stageHeight,
    borderRadius: "var(--riff-radius, 7px)",
    border: "1px dashed var(--riff-border)",
    background: "var(--riff-subtle)",
    color: "var(--riff-muted)",
    boxSizing: "border-box",
    padding: "6px 10px",
    fontFamily: "var(--riff-font)",
    fontSize: "11.5px",
    lineHeight: "16px",
  };

  return (
    <ContextMenu title={title} width={width} items={mapItems(items)}>
      <div ref={areaRef} style={areaStyle}>
        Right-click area
      </div>
    </ContextMenu>
  );
}

export default function cell(props) {
  return <ContextMenuCell {...props} />;
}
