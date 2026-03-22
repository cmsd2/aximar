import { useState, useCallback, useRef } from "react";
import { getFunction } from "../lib/catalog-client";
import { parseSignature, type ParsedSignature } from "../lib/signature-parser";
import { findEnclosingCall, getParamIndex } from "../lib/param-tracker";
export interface SignatureHintState {
  visible: boolean;
  signatures: ParsedSignature[];
  activeParamIndex: number | null;
  anchorPosition: number;
}

const INITIAL_STATE: SignatureHintState = {
  visible: false,
  signatures: [],
  activeParamIndex: null,
  anchorPosition: 0,
};

export function useSignatureHint(
  textareaRef: React.RefObject<HTMLTextAreaElement | null>,
  mode: "hint" | "active-hint"
) {
  const [state, setState] = useState<SignatureHintState>(INITIAL_STATE);
  const funcCacheRef = useRef<Map<string, ParsedSignature[]>>(new Map());

  const show = useCallback(
    async (funcName: string, openParenPos: number) => {
      // Check cache first
      let signatures = funcCacheRef.current.get(funcName);
      if (!signatures) {
        const func = await getFunction(funcName);
        if (!func || func.signatures.length === 0) return;
        signatures = func.signatures.map(parseSignature);
        // Skip if all signatures have no params
        if (signatures.every((s) => s.params.length === 0)) return;
        funcCacheRef.current.set(funcName, signatures);
      }

      setState({
        visible: true,
        signatures,
        activeParamIndex: mode === "active-hint" ? 0 : null,
        anchorPosition: openParenPos,
      });
    },
    [mode]
  );

  const update = useCallback(() => {
    if (mode !== "active-hint") return;
    const textarea = textareaRef.current;
    if (!textarea) return;

    const text = textarea.value;
    const cursorPos = textarea.selectionStart;
    const call = findEnclosingCall(text, cursorPos);

    if (!call) {
      setState(INITIAL_STATE);
      return;
    }

    const paramIdx = getParamIndex(text, call.openParenPos, cursorPos);
    if (paramIdx === null) {
      setState(INITIAL_STATE);
      return;
    }

    // If the function changed, fetch new signatures
    const cached = funcCacheRef.current.get(call.funcName);
    if (cached) {
      setState((s) => ({
        ...s,
        visible: true,
        signatures: cached,
        activeParamIndex: paramIdx,
        anchorPosition: call.openParenPos,
      }));
    } else {
      // Fetch async
      getFunction(call.funcName).then((func) => {
        if (!func || func.signatures.length === 0) {
          setState(INITIAL_STATE);
          return;
        }
        const signatures = func.signatures.map(parseSignature);
        if (signatures.every((s) => s.params.length === 0)) {
          setState(INITIAL_STATE);
          return;
        }
        funcCacheRef.current.set(call.funcName, signatures);
        setState({
          visible: true,
          signatures,
          activeParamIndex: paramIdx,
          anchorPosition: call.openParenPos,
        });
      });
    }
  }, [mode, textareaRef]);

  const dismiss = useCallback(() => {
    setState(INITIAL_STATE);
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (!state.visible) return false;
      if (e.key === "Escape") {
        e.preventDefault();
        dismiss();
        return true;
      }
      return false;
    },
    [state.visible, dismiss]
  );

  return { state, show, update, dismiss, handleKeyDown };
}
