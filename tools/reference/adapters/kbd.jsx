// Maps a stories/kbd.json cell to riff-uiux's <Kbd>.

import { Kbd } from "@riff/component";

export default function cell({ label, ...props }) {
  return <Kbd {...props}>{label}</Kbd>;
}
