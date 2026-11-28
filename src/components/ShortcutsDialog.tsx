import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { COMMANDS, type MenuName } from "../commands/registry";
import { TITLES } from "../commands/menu-model";
import { formatShortcut, type Shortcut } from "../commands/shortcut";
import { Button } from "./ui/Button";
import { Dialog } from "./ui/Dialog";

const MENUS: MenuName[] = ["file", "edit", "cell", "session", "view", "window", "help"];

/** Keys the cell editor and the window handle themselves, not the menu */
const EDITOR_KEYS: { keys: Shortcut[]; label: MessageDescriptor }[] = [
  {
    keys: ["Shift+Enter"],
    label: msg({ message: "Run the cell and move on", comment: "Keyboard Shortcuts list: what Shift+Enter does in a cell" }),
  },
  {
    keys: ["Mod+Enter"],
    label: msg({ message: "Run the cell and stay in it", comment: "Keyboard Shortcuts list: what Cmd+Enter or Ctrl+Enter does in a cell" }),
  },
  {
    keys: ["ArrowUp", "ArrowDown"],
    label: msg({
      message: "From a cell's first or last line, to the gap beside it, then the next cell",
      comment: "Keyboard Shortcuts list: what the Up and Down arrows do at the edge of a cell",
    }),
  },
  {
    keys: ["Tab"],
    label: msg({
      message: "At the caret between cells, open the + menu: a code or Markdown cell, or a next step for the result above",
      comment: "Keyboard Shortcuts list: what Tab does at the caret between two cells",
    }),
  },
  {
    keys: ["Escape"],
    label: msg({ message: "Leave the cell, or remove the caret between cells", comment: "Keyboard Shortcuts list: what Escape does" }),
  },
  {
    keys: ["Ctrl+Tab", "Ctrl+Shift+Tab"],
    label: msg({ message: "Next and previous tab", comment: "Keyboard Shortcuts list: what Ctrl+Tab and Ctrl+Shift+Tab do" }),
  },
];

/** Help > Keyboard Shortcuts: every command with a shortcut, by menu, then the editor's own keys */
export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  const { t, i18n } = useLingui();
  return (
    <Dialog
      size="md"
      title={t({ message: "Keyboard Shortcuts", comment: "Title of the dialog that lists every keyboard shortcut" })}
      onClose={onClose}
      footer={
        <Button variant="primary" onClick={onClose}>
          {t({ message: "Done", comment: "Keyboard Shortcuts dialog: the button that closes it. max-length:12" })}
        </Button>
      }
    >
      <div className="shortcuts-list">
        {MENUS.map((menu) => {
          const commands = COMMANDS.filter(
            (c) => c.shortcut && (c.menu?.name === menu || (menu === "edit" && c.id === "app.settings")),
          );
          if (commands.length === 0) return null;
          return (
            <section key={menu} className="shortcuts-group">
              <h3>{i18n._(TITLES[menu])}</h3>
              <dl>
                {commands.map((c) => (
                  <div key={c.id} className="shortcuts-row" data-command={c.id}>
                    <dt>{i18n._(c.label)}</dt>
                    <dd>
                      <kbd>{formatShortcut(c.shortcut!)}</kbd>
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          );
        })}
        <section className="shortcuts-group">
          <h3>{t({ message: "In cells", comment: "Keyboard Shortcuts dialog: heading of the keys the cell editor handles itself" })}</h3>
          <dl>
            {EDITOR_KEYS.map((k) => (
              <div key={k.keys.join()} className="shortcuts-row">
                <dt>{i18n._(k.label)}</dt>
                <dd>
                  {k.keys.map((key) => (
                    <kbd key={key}>{formatShortcut(key)}</kbd>
                  ))}
                </dd>
              </div>
            ))}
          </dl>
        </section>
      </div>
    </Dialog>
  );
}
