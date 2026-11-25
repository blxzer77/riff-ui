// Maps a stories/command-menu.json cell to riff-uiux's <CommandMenu>.

import { useEffect, useRef } from "react";
import { CommandMenu } from "@riff/component";
import { Icon } from "@riff/gallery";

export const freezeCss = `
  .ui-cm *, .ui-cm *::before, .ui-cm *::after {
    animation: none !important;
    transition: none !important;
  }
`;

function mapGroups(groups) {
  if (!groups) return undefined;
  return groups.map((g) => ({
    id: g.id,
    label: g.label,
    commands: (g.commands || []).map((c) => ({
      id: c.id,
      label: c.label,
      hint: c.hint,
      keywords: c.keywords,
      keys: c.keys,
      icon: c.icon ? <Icon name={c.icon} size="100%" /> : undefined,
    })),
  }));
}

function CommandMenuCell({
  width = 560,
  placeholder,
  groups,
  query,
  clickLabel,
  selectedId,
}) {
  const hostRef = useRef(null);

  useEffect(() => {
    const root = hostRef.current;
    if (!root) return;

    if (query) {
      const input = root.querySelector("input");
      if (input) {
        const setter = Object.getOwnPropertyDescriptor(
          HTMLInputElement.prototype,
          "value",
        ).set;
        setter.call(input, query);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }

    if (clickLabel || selectedId) {
      const targetText = clickLabel;
      const buttons = Array.from(root.querySelectorAll("button.ui-cm__item"));
      const btn = targetText
        ? buttons.find((b) =>
            b.querySelector(".ui-cm__name")?.textContent?.includes(targetText),
          )
        : undefined;
      if (btn) {
        btn.click();
      }
    }
  }, [query, clickLabel, selectedId]);

  return (
    <div ref={hostRef} style={{ width }}>
      <CommandMenu
        autoFocus={false}
        placeholder={placeholder}
        groups={mapGroups(groups)}
      />
    </div>
  );
}

export default function cell(props) {
  return <CommandMenuCell {...props} />;
}
