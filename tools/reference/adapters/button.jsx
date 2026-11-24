// Maps a stories/button.json cell to riff-uiux's <Button>.
//
// Story props are plain data shared with the GPUI side; this is the only place
// that knows how riff-uiux spells them.

import { Button } from "@riff/component";
import { Icon } from "@riff/gallery";

/** Holds the loading sweep and spinner `freeze_ms` into their cycle (the GPUI
 *  side freezes at the same moment) and skips enter animations, so the
 *  pictures repeat. `@@FREEZE_MS@@` is filled in from the story. */
export const freezeCss = `
  .ui-button__label, .ui-button__icon { animation: none !important; }
  .ui-button__label::after, .ui-button__spinner {
    animation-play-state: paused !important;
    animation-delay: -@@FREEZE_MS@@ms !important;
  }
`;

export default function cell({ label, icon, iconRight, ariaLabel, ...props }) {
  return (
    <Button
      {...props}
      icon={icon ? <Icon name={icon} size="100%" /> : undefined}
      iconRight={iconRight ? <Icon name={iconRight} size="100%" /> : undefined}
      aria-label={ariaLabel}
    >
      {label}
    </Button>
  );
}
