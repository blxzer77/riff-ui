// Maps a stories/badge.json cell to riff-uiux's <Badge>.

import { Badge } from "@riff/component";
import { Icon } from "@riff/gallery";

export const freezeCss = `
  .ui-bdg__dot[data-live]::after {
    animation-play-state: paused !important;
    animation-delay: -@@FREEZE_MS@@ms !important;
  }
`;

export default function cell({ label, icon, ...props }) {
  return (
    <Badge
      {...props}
      icon={icon ? <Icon name={icon} size="100%" /> : undefined}
    >
      {label}
    </Badge>
  );
}
