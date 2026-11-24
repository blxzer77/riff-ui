// Maps a stories/gradient-text.json cell to riff-uiux's <GradientText>.

import { GradientText } from "@riff/component";

export const freezeCss = `
  .ui-gt__fill {
    animation-play-state: paused !important;
    animation-delay: -@@FREEZE_MS@@ms !important;
  }
`;

export default function cell({
  text,
  prefix,
  colors,
  speed,
  sweep,
  as,
  fontSize = 14,
  fontWeight = 400,
  lineHeight = 20,
}) {
  const style = {
    fontSize: `${fontSize}px`,
    fontWeight,
    lineHeight: `${lineHeight}px`,
    color: "var(--riff-ink)",
  };

  const gt = (
    <GradientText colors={colors} speed={speed} sweep={sweep} as={as}>
      {text}
    </GradientText>
  );

  if (prefix) {
    return (
      <span style={{ ...style, display: "inline-flex", alignItems: "baseline" }}>
        <span style={{ whiteSpace: "pre" }}>{prefix}</span>
        {gt}
      </span>
    );
  }

  return <span style={{ ...style, display: "inline-flex" }}>{gt}</span>;
}
