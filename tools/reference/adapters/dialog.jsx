// Maps a stories/dialog.json cell to riff-uiux's <Dialog> or <AlertDialog>.

import { AlertDialog, Dialog } from "@riff/component";

export const freezeCss = `
  .ui-dlg, .ui-dlg__panel, .ui-dlg__spin { animation: none !important; }
`;

function FooterButtons({ cancelLabel, confirmLabel }) {
  const btn = (primary) => ({
    height: 32,
    padding: "0 12px",
    borderRadius: "calc(var(--riff-radius, 7px) - 2px)",
    border: primary ? "1px solid transparent" : "1px solid var(--riff-border)",
    background: primary ? "var(--riff-primary)" : "var(--riff-surface)",
    color: primary ? "var(--riff-on-primary)" : "var(--riff-ink)",
    fontFamily: "var(--riff-font)",
    fontSize: "13px",
    lineHeight: "18px",
    fontWeight: 500,
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
  });
  return (
    <>
      {cancelLabel && (
        <button type="button" style={btn(false)}>
          {cancelLabel}
        </button>
      )}
      {confirmLabel && (
        <button type="button" style={btn(true)}>
          {confirmLabel}
        </button>
      )}
    </>
  );
}

export default function cell({
  alert = false,
  size,
  width,
  title,
  description,
  body,
  closeButton = true,
  footerCancel,
  footerConfirm,
  cancelLabel = "Cancel",
  confirmLabel = "Confirm",
  danger = false,
  stageWidth = 450,
  stageHeight = 230,
}) {
  const frameStyle = {
    position: "relative",
    width: stageWidth,
    height: stageHeight,
    overflow: "hidden",
    borderRadius: "var(--riff-radius, 7px)",
  };

  if (alert) {
    return (
      <div style={frameStyle}>
        <AlertDialog
          contained
          open
          size={size}
          width={width}
          title={title}
          description={description}
          cancelLabel={cancelLabel}
          confirmLabel={confirmLabel}
          danger={danger}
        >
          {body}
        </AlertDialog>
      </div>
    );
  }

  const hasFooter = footerCancel || footerConfirm;
  return (
    <div style={frameStyle}>
      <Dialog
        contained
        open
        size={size}
        width={width}
        title={title}
        description={description}
        closeButton={closeButton}
        footer={
          hasFooter ? (
            <FooterButtons cancelLabel={footerCancel} confirmLabel={footerConfirm} />
          ) : undefined
        }
      >
        {body}
      </Dialog>
    </div>
  );
}
