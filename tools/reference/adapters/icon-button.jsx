// Maps a stories/icon-button.json cell to riff-uiux's <IconButton>.

import { IconButton } from "@riff/component";
import { Icon } from "@riff/gallery";

export default function cell({ icon, label = "Action", ...props }) {
  return (
    <IconButton
      {...props}
      icon={icon ? <Icon name={icon} size="100%" /> : undefined}
      label={label}
    />
  );
}
