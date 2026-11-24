// Maps a stories/avatar.json cell to riff-uiux's <Avatar> and <AvatarGroup>.

import { Avatar, AvatarGroup } from "@riff/component";

const SAMPLE_PHOTO =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFAAAABQCAYAAACOEfKtAAACc0lEQVR42u2b2U7bQBSGj1OeplelQCmoZW8RQlEU0jghZithp4HEIMQjINRCxb4EiAOkIaxCXFVAgcfgYUgvIsRtiO1Z7P+XfOeRZz6d499zzlh5O/OY99ATeSgv5PVG0Hm9zA0ypTJSAAEAARAAXSuYCCIQAAEQACEAhAsjAgEQAkAAhIkgAiEABEAALE1zU1+Lvnfx1zkAPuun3vLqMYkpPxERLS+cutuFF/RmU+NjeoAzQIWI1/Vbb7JkEeP6N25r4AZwKdFoaSSMJoLuAbgSb7AlnYbjqvMBrk3W2/pOGpwMMV2PM78DFZYmwlAbsTomz4nGwtiJyBKFAAiAACg1QBRUEYFIYQAEQAAEQFe48EjygclzdpJZuLA8xQTG5ayx3XtbF7S9e+T8guqPvTtb4G3u5dxT0p9I/bMU3nrq2F09EVKI4satJfBWjROOTSXO0o0bU+OXDb59YSFceDp9TR7K03zPl6LHLKbPC7+dcp6/UJ8xs/t/i/9fWJB5i3G0o6ul9BQ+cGkKL2nWNNZjXS9HOzYPcxwAMtaaZl9feFhTC1u5Qwdu5bYin5ktKqoV2ppGJuOMrVwy8onLa6I3EpH7ZEIqXMvdoLo7NSIi+pPdlysC0+EaoQo0neFuebZyB6GPQla5QqEe8bdyGbVa6FJhUO0VN4Wz6gcp6q0BtU+8FM4Fq0gm+YPfcT7QtBRBUvgkUCklP1+gn38Kn3VUSB2E3o6o3AVV2WUqAi/87x0Bod0/ABPhZSglR+Clr9xR/Np8g4hAHlEIE+FhIlfed46E0eodQgqzTmMABEAAlBogXNikEIFIYQAEQACESgb4H8ajwV1bpLGQAAAAAElFTkSuQmCC";

export const freezeCss = `
  .ui-avt[data-working] .ui-avt__mark i {
    animation-play-state: paused !important;
  }
  .ui-avt[data-working] .ui-avt__mark i:nth-child(1) { animation-delay: calc(0ms - @@FREEZE_MS@@ms) !important; }
  .ui-avt[data-working] .ui-avt__mark i:nth-child(2),
  .ui-avt[data-working] .ui-avt__mark i:nth-child(4) { animation-delay: calc(120ms - @@FREEZE_MS@@ms) !important; }
  .ui-avt[data-working] .ui-avt__mark i:nth-child(3),
  .ui-avt[data-working] .ui-avt__mark i:nth-child(5),
  .ui-avt[data-working] .ui-avt__mark i:nth-child(7) { animation-delay: calc(240ms - @@FREEZE_MS@@ms) !important; }
  .ui-avt[data-working] .ui-avt__mark i:nth-child(6),
  .ui-avt[data-working] .ui-avt__mark i:nth-child(8) { animation-delay: calc(360ms - @@FREEZE_MS@@ms) !important; }
  .ui-avt[data-working] .ui-avt__mark i:nth-child(9) { animation-delay: calc(480ms - @@FREEZE_MS@@ms) !important; }
  .ui-avt__img {
    transition: none !important;
  }
`;

function renderOne({ photo, broken, ...rest }, key) {
  const src = broken ? "invalid://missing.png" : photo ? SAMPLE_PHOTO : undefined;
  return <Avatar key={key} src={src} {...rest} />;
}

export default function cell({ group, max, total, size, ...rest }) {
  if (group) {
    return (
      <AvatarGroup max={max} total={total} size={size}>
        {group.map((item, i) => renderOne(item, i))}
      </AvatarGroup>
    );
  }
  return renderOne({ size, ...rest });
}
