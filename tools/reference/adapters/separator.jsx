// Maps a stories/separator.json cell to riff-uiux's <Separator>.

import { Separator } from "@riff/component";

export default function cell({ width, height, ...props }) {
  const style = height != null ? { height } : undefined;
  const sep = <Separator style={style} {...props} />;
  if (width != null) {
    return <div style={{ width, display: "flex" }}>{sep}</div>;
  }
  return sep;
}
