import { StateField, StateEffect, type EditorState } from "@codemirror/state";
import { showTooltip, type Tooltip } from "@codemirror/view";
import type { EditorView } from "@codemirror/view";
import { getFunction, packageForFunction, getPackage } from "./catalog-client";
import { parseSignature, type ParsedSignature } from "./signature-parser";
import { findEnclosingCall, getParamIndex } from "./param-tracker";

interface SignatureHintData {
  signatures: ParsedSignature[];
  anchorPos: number;
  activeParamIndex: number | null;
  funcName: string;
}

export const showSignatureEffect = StateEffect.define<SignatureHintData>();
export const hideSignatureEffect = StateEffect.define<void>();
export const updateParamIndexEffect = StateEffect.define<number | null>();

const signatureHintField = StateField.define<SignatureHintData | null>({
  create() {
    return null;
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(showSignatureEffect)) return effect.value;
      if (effect.is(hideSignatureEffect)) return null;
      if (effect.is(updateParamIndexEffect) && value) {
        return { ...value, activeParamIndex: effect.value };
      }
    }
    return value;
  },
  provide(field) {
    return showTooltip.computeN([field], (state: EditorState) => {
      const data = state.field(field);
      if (!data) return [];
      return [signatureTooltip(data, state)];
    });
  },
});

function signatureTooltip(data: SignatureHintData, state: EditorState): Tooltip {
  const pos = Math.min(data.anchorPos, state.doc.length);
  return {
    pos,
    above: true,
    create() {
      const dom = renderSignatureHint(data);
      return { dom };
    },
  };
}

const MAX_SHOWN = 5;

function renderSignatureHint(data: SignatureHintData): HTMLElement {
  const container = document.createElement("div");
  container.className = "signature-hint";

  const shown = data.signatures.slice(0, MAX_SHOWN);
  const overflow = data.signatures.length - MAX_SHOWN;

  for (const sig of shown) {
    const exceeded =
      data.activeParamIndex !== null &&
      sig.params.length > 0 &&
      data.activeParamIndex >= sig.params.length;

    const line = document.createElement("div");
    line.className = `signature-hint-line${exceeded ? " sig-hint-overflow" : ""}`;

    const nameSpan = document.createElement("span");
    nameSpan.className = "signature-hint-name";
    nameSpan.textContent = sig.name;
    line.appendChild(nameSpan);

    const openParen = document.createElement("span");
    openParen.className = "signature-hint-parens";
    openParen.textContent = "(";
    line.appendChild(openParen);

    sig.params.forEach((param, pi) => {
      if (pi > 0) {
        const comma = document.createElement("span");
        comma.className = "signature-hint-comma";
        comma.textContent = ", ";
        line.appendChild(comma);
      }
      const paramSpan = document.createElement("span");
      paramSpan.className =
        data.activeParamIndex === pi ? "sig-hint-active" : "signature-hint-param";
      paramSpan.textContent = param;
      line.appendChild(paramSpan);
    });

    const closeParen = document.createElement("span");
    closeParen.className = "signature-hint-parens";
    closeParen.textContent = ")";
    line.appendChild(closeParen);

    container.appendChild(line);
  }

  if (overflow > 0) {
    const more = document.createElement("div");
    more.className = "signature-hint-more";
    more.textContent = `+${overflow} more`;
    container.appendChild(more);
  }

  return container;
}

// Cache for function signatures
const sigCache = new Map<string, ParsedSignature[]>();

/** Look up signatures for a function from the catalog, falling back to package signatures. */
async function resolveSignatures(funcName: string): Promise<string[]> {
  const func = await getFunction(funcName);
  if (func && func.signatures.length > 0) return func.signatures;

  // Fallback: check package functions
  const pkgName = await packageForFunction(funcName);
  if (pkgName) {
    const pkg = await getPackage(pkgName);
    if (pkg?.signatures?.[funcName]) {
      return [pkg.signatures[funcName]];
    }
  }
  return [];
}

export async function triggerSignatureHint(
  view: EditorView,
  funcName: string,
  openParenPos: number,
  mode: "hint" | "active-hint" | "snippet"
) {
  let signatures = sigCache.get(funcName);
  if (!signatures) {
    const rawSigs = await resolveSignatures(funcName);
    if (rawSigs.length === 0) return;
    signatures = rawSigs.map(parseSignature);
    if (signatures.every((s) => s.params.length === 0)) return;
    sigCache.set(funcName, signatures);
  }

  view.dispatch({
    effects: showSignatureEffect.of({
      signatures,
      anchorPos: openParenPos,
      activeParamIndex: mode === "active-hint" ? 0 : null,
      funcName,
    }),
  });
}

export function updateSignatureHint(view: EditorView) {
  const text = view.state.doc.toString();
  const cursorPos = view.state.selection.main.head;
  const current = view.state.field(signatureHintField);

  if (!current) return;

  const call = findEnclosingCall(text, cursorPos);
  if (!call) {
    view.dispatch({ effects: hideSignatureEffect.of(undefined) });
    return;
  }

  const paramIdx = getParamIndex(text, call.openParenPos, cursorPos);
  if (paramIdx === null) {
    view.dispatch({ effects: hideSignatureEffect.of(undefined) });
    return;
  }

  // If the function changed, fetch new signatures
  if (call.funcName !== current.funcName) {
    const cached = sigCache.get(call.funcName);
    if (cached) {
      view.dispatch({
        effects: showSignatureEffect.of({
          signatures: cached,
          anchorPos: call.openParenPos,
          activeParamIndex: paramIdx,
          funcName: call.funcName,
        }),
      });
    } else {
      resolveSignatures(call.funcName).then((rawSigs) => {
        if (rawSigs.length === 0) {
          view.dispatch({ effects: hideSignatureEffect.of(undefined) });
          return;
        }
        const signatures = rawSigs.map(parseSignature);
        if (signatures.every((s) => s.params.length === 0)) {
          view.dispatch({ effects: hideSignatureEffect.of(undefined) });
          return;
        }
        sigCache.set(call.funcName, signatures);
        view.dispatch({
          effects: showSignatureEffect.of({
            signatures,
            anchorPos: call.openParenPos,
            activeParamIndex: paramIdx,
            funcName: call.funcName,
          }),
        });
      });
    }
  } else {
    view.dispatch({ effects: updateParamIndexEffect.of(paramIdx) });
  }
}

export { signatureHintField };
