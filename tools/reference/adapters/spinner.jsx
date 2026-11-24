// Maps a stories/spinner.json cell to riff-uiux's <Spinner>.

import { Spinner } from "@riff/component";

const COLOR = {
  ink: "var(--riff-ink)",
  muted: "var(--riff-muted)",
  info: "var(--riff-info)",
  success: "var(--riff-success)",
  warning: "var(--riff-warning)",
  danger: "var(--riff-danger)",
  purple: "var(--riff-purple)",
};

export const freezeCss = `
  .ui-spn {
    animation-play-state: paused !important;
    animation-delay: -@@FREEZE_MS@@ms !important;
  }
`;

export default function cell({ color, speedMs, ...props }) {
  const style = {
    color: COLOR[color ?? "ink"],
    ...(speedMs != null ? { "--ui-spn-speed": `${speedMs}ms` } : {}),
  };
  return <Spinner style={style} {...props} />;
}
