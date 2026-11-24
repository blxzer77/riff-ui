// Maps a stories/card.json cell to riff-uiux's <Card>.

import { Card, CardContent, CardFooter, CardHeader } from "@riff/component";
import { Icon } from "@riff/gallery";

export default function cell({
  width,
  interactive,
  selected,
  icon,
  title,
  description,
  actionIcon,
  headerExtra,
  content,
  footer,
}) {
  const hasHeader = icon || title || description || actionIcon || headerExtra;
  const titleNode =
    title && interactive ? (
      <button type="button" aria-pressed={selected || undefined}>
        {title}
      </button>
    ) : (
      title
    );

  return (
    <Card
      interactive={interactive}
      selected={selected}
      style={width ? { width: `${width}px` } : undefined}
    >
      {hasHeader && (
        <CardHeader
          icon={icon ? <Icon name={icon} size={16} /> : undefined}
          title={titleNode}
          description={description}
          action={actionIcon ? <Icon name={actionIcon} size={16} /> : undefined}
        >
          {headerExtra ? (
            <div style={{ fontSize: "12px", lineHeight: "16px", marginTop: "4px" }}>
              {headerExtra}
            </div>
          ) : undefined}
        </CardHeader>
      )}
      {content && <CardContent>{content}</CardContent>}
      {footer && <CardFooter>{footer}</CardFooter>}
    </Card>
  );
}
