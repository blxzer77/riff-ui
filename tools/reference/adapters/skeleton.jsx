// Maps a stories/skeleton.json cell to riff-uiux's <Skeleton> and <Loading>.

import { Loading, Skeleton } from "@riff/component";

export const freezeCss = `
  .ui-skl::after {
    animation-play-state: paused !important;
    animation-delay: -@@FREEZE_MS@@ms !important;
  }
  .ui-skl-loading {
    animation-play-state: paused !important;
  }
`;

export default function cell({
  containerWidth,
  width,
  height,
  radius,
  circle,
  lineHeight,
  preset,
  loadingAtMs,
}) {
  if (preset === "row") {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: 10, width: 220 }}>
        <Skeleton width={16} height={16} radius={4} />
        <Skeleton line={20} width={164} height={10} />
      </div>
    );
  }
  if (preset === "loading") {
    return (
      <Loading
        label="Loading runs"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
          width: 180,
          animationDelay: `${150 - (loadingAtMs ?? 600)}ms`,
        }}
      >
        <Skeleton line={20} width={160} height={10} />
        <Skeleton line={20} width={120} height={10} />
      </Loading>
    );
  }

  const skl = (
    <Skeleton
      width={width}
      height={height}
      radius={circle ? "50%" : radius}
      line={lineHeight}
    />
  );
  if (containerWidth != null) {
    return <div style={{ width: containerWidth, display: "flex" }}>{skl}</div>;
  }
  return skl;
}
