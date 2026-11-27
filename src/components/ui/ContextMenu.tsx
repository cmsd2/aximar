import { useEffect, useLayoutEffect, useRef, useState } from "react";

export interface ContextMenuItem {
  id: string;
  label: string;
  /** Shown on the right, already in the platform's names */
  shortcut?: string;
  disabled?: boolean;
  /** Start a new group: a separator goes above this item */
  separatorBefore?: boolean;
  onChoose: () => void;
}

/**
 * A menu at the pointer: Up and Down move between the enabled items,
 * Enter or a click chooses one, and Escape, a click outside or a scroll
 * closes it.  It keeps inside the window.
 */
export function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: ContextMenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  // Inside the window, measured once drawn
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      left: Math.max(0, Math.min(x, window.innerWidth - width)),
      top: Math.max(0, Math.min(y, window.innerHeight - height)),
    });
  }, [x, y]);

  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    const away = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const scroll = () => onClose();
    document.addEventListener("mousedown", away);
    window.addEventListener("scroll", scroll, true);
    window.addEventListener("blur", onClose);
    return () => {
      document.removeEventListener("mousedown", away);
      window.removeEventListener("scroll", scroll, true);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);

  const keys = (e: React.KeyboardEvent) => {
    const buttons = Array.from(ref.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : buttons.length - 1;
      buttons[(i + step) % buttons.length]?.focus();
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      buttons[e.key === "Home" ? 0 : buttons.length - 1]?.focus();
    }
  };

  return (
    <div
      ref={ref}
      className="cell-gap-menu context-menu"
      role="menu"
      style={{ position: "fixed", left: pos.left, top: pos.top }}
      onKeyDown={keys}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item) => (
        <div key={item.id} role="none">
          {item.separatorBefore && <div className="context-menu-separator" role="separator" />}
          <button
            role="menuitem"
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onChoose();
            }}
          >
            <span>{item.label}</span>
            {item.shortcut && <kbd className="context-menu-key">{item.shortcut}</kbd>}
          </button>
        </div>
      ))}
    </div>
  );
}
