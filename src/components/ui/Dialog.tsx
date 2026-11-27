import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";

/**
 * The dim layer behind a dialog.  A click on it, outside the dialog,
 * closes the dialog.
 */
export function DialogOverlay({ onClose, children }: { onClose: () => void; children: ReactNode }) {
  return (
    <div
      className="ui-overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {children}
    </div>
  );
}

/**
 * A dialog's frame, for dialogs with their own layout such as Settings.
 * Takes focus when it opens and gives it back when it closes.  Esc
 * closes it, except in a menu, where Esc closes the menu.
 */
export function DialogFrame({
  onClose,
  size = "md",
  labelledBy,
  className,
  children,
}: {
  onClose: () => void;
  size?: "sm" | "md" | "lg";
  /** The id of the dialog's title */
  labelledBy: string;
  className?: string;
  children: ReactNode;
}) {
  const frame = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    if (!frame.current?.contains(document.activeElement)) frame.current?.focus();
    return () => opener?.focus?.();
  }, []);

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    if ((e.target as Element).tagName === "SELECT") return;
    e.stopPropagation();
    onClose();
  };

  return (
    <div
      ref={frame}
      className={["ui-dialog", `ui-dialog--${size}`, className].filter(Boolean).join(" ")}
      role="dialog"
      aria-modal="true"
      aria-labelledby={labelledBy}
      tabIndex={-1}
      onKeyDown={onKeyDown}
    >
      {children}
    </div>
  );
}

/** A dialog with a title, a body and a footer of buttons */
export function Dialog({
  title,
  onClose,
  footer,
  size = "sm",
  children,
}: {
  /** Already translated */
  title: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  size?: "sm" | "md" | "lg";
  children: ReactNode;
}) {
  const titleId = useId();
  return (
    <DialogOverlay onClose={onClose}>
      <DialogFrame onClose={onClose} size={size} labelledBy={titleId}>
        <header className="ui-dialog-header">
          <h2 className="ui-dialog-title" id={titleId}>
            {title}
          </h2>
        </header>
        <div className="ui-dialog-body">{children}</div>
        {footer && <footer className="ui-dialog-footer">{footer}</footer>}
      </DialogFrame>
    </DialogOverlay>
  );
}
