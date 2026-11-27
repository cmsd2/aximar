// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, useState } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { Button } from "./Button";
import { Dialog } from "./Dialog";
import { IconX } from "./icons";
import { SegmentedControl } from "./SegmentedControl";
import { mount, settle, unmount } from "../settings/settings-test-kit";

afterEach(unmount);

async function key(el: Element, k: string) {
  await act(async () => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
  });
  await settle();
}

describe("Button", () => {
  it("button_variants_render_their_classes", async () => {
    const container = await mount(
      <>
        <Button variant="primary">A</Button>
        <Button>B</Button>
        <Button variant="quiet" size="sm">
          C
        </Button>
        <Button variant="quiet" icon={IconX} label="Close" />
      </>,
    );
    const classes = [...container.querySelectorAll("button")].map((b) => b.className);
    expect(classes).toEqual([
      "ui-button ui-button--primary ui-button--md",
      "ui-button ui-button--secondary ui-button--md",
      "ui-button ui-button--quiet ui-button--sm",
      "ui-button ui-button--quiet ui-button--icon",
    ]);
    const iconOnly = container.querySelectorAll("button")[3];
    expect(iconOnly.getAttribute("aria-label")).toBe("Close");
    expect(iconOnly.getAttribute("type")).toBe("button");
  });

  it("icon_only_button_needs_a_label", () => {
    // Checked by tsc: an icon-only button without a label doesn't compile
    // @ts-expect-error label is required when there is no text
    const missing = <Button icon={IconX} />;
    expect(missing).toBeTruthy();
  });
});

describe("SegmentedControl", () => {
  function Demo({ onChange }: { onChange: (v: string) => void }) {
    const [value, setValue] = useState("b");
    return (
      <SegmentedControl
        options={[
          { value: "a", label: "A" },
          { value: "b", label: "B" },
          { value: "c", label: "C" },
        ]}
        value={value}
        onChange={(v) => {
          setValue(v);
          onChange(v);
        }}
      />
    );
  }

  it("segmented_arrow_keys_move_the_choice", async () => {
    const changes: string[] = [];
    const container = await mount(<Demo onChange={(v) => changes.push(v)} />);
    const checked = () => container.querySelector('[aria-checked="true"]')?.textContent;
    expect(checked()).toBe("B");
    await key(container.querySelector('[aria-checked="true"]')!, "ArrowRight");
    expect(checked()).toBe("C");
    expect(document.activeElement?.textContent).toBe("C");
    await key(document.activeElement!, "Home");
    expect(checked()).toBe("A");
    await key(document.activeElement!, "ArrowLeft");
    expect(checked()).toBe("A");
    expect(changes).toEqual(["c", "a", "a"]);
  });

  it("gives only the chosen option a tab stop", async () => {
    const container = await mount(<Demo onChange={() => {}} />);
    const stops = [...container.querySelectorAll("button")].map((b) => b.tabIndex);
    expect(stops).toEqual([-1, 0, -1]);
  });
});

describe("Dialog", () => {
  function Opener({ onClose }: { onClose: () => void }) {
    const [open, setOpen] = useState(false);
    return (
      <>
        <button id="opener" onClick={() => setOpen(true)}>
          Open
        </button>
        {open && (
          <Dialog
            title="Title"
            onClose={() => {
              onClose();
              setOpen(false);
            }}
          >
            <select>
              <option>x</option>
            </select>
          </Dialog>
        )}
      </>
    );
  }

  async function openDialog() {
    const onClose = vi.fn();
    const container = await mount(<Opener onClose={onClose} />);
    const opener = container.querySelector<HTMLButtonElement>("#opener")!;
    opener.focus();
    await act(async () => opener.click());
    await settle();
    return { onClose, opener };
  }

  it("esc_closes", async () => {
    const { onClose } = await openDialog();
    await key(document.querySelector('[role="dialog"]')!, "Escape");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("leaves Esc in a menu to the menu", async () => {
    const { onClose } = await openDialog();
    await key(document.querySelector('[role="dialog"] select')!, "Escape");
    expect(onClose).not.toHaveBeenCalled();
  });

  it("a_click_outside_closes", async () => {
    const { onClose } = await openDialog();
    await act(async () => {
      document.querySelector('[role="dialog"]')!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    });
    expect(onClose).not.toHaveBeenCalled();
    await act(async () => {
      document.querySelector(".ui-overlay")!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("focus_returns_to_the_opener", async () => {
    const { opener } = await openDialog();
    expect(document.activeElement).toBe(document.querySelector('[role="dialog"]'));
    await key(document.querySelector('[role="dialog"]')!, "Escape");
    expect(document.activeElement).toBe(opener);
  });

  it("names itself by its title", async () => {
    await openDialog();
    const dialog = document.querySelector('[role="dialog"]')!;
    expect(document.getElementById(dialog.getAttribute("aria-labelledby")!)?.textContent).toBe("Title");
  });
});
