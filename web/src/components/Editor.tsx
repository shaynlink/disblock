import * as Blockly from "blockly";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api.ts";
import { createTheme, registerBlocks, starterWorkspace, toolbox } from "../blocks/blocks.ts";
import { generateProgram } from "../blocks/generate.ts";
import type { BotDetail } from "../types.ts";

type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "error"; message: string };

function hasBlocks(workspace: object): boolean {
  return "blocks" in workspace;
}

export function Editor({ bot }: { bot: BotDetail }) {
  const container = useRef<HTMLDivElement>(null);
  const workspace = useRef<Blockly.WorkspaceSvg | null>(null);
  const savedSnapshot = useRef("");
  const [dirty, setDirty] = useState(false);
  const [save, setSave] = useState<SaveState>({ kind: "idle" });

  useEffect(() => {
    const element = container.current;
    if (!element) {
      return;
    }
    registerBlocks();
    const editor = Blockly.inject(element, {
      toolbox,
      theme: createTheme(),
      renderer: "zelos",
      trashcan: true,
      grid: { spacing: 24, length: 2, colour: "#2b2d31", snap: true },
      zoom: { controls: true, wheel: true, startScale: 0.85, maxScale: 2, minScale: 0.4 },
      move: { scrollbars: true, drag: true, wheel: false },
    });
    workspace.current = editor;

    Blockly.serialization.workspaces.load(
      hasBlocks(bot.workspace) ? bot.workspace : starterWorkspace,
      editor,
    );
    // The starter template counts as unsaved, so the first save registers it.
    savedSnapshot.current = hasBlocks(bot.workspace)
      ? JSON.stringify(Blockly.serialization.workspaces.save(editor))
      : "";
    setDirty(!hasBlocks(bot.workspace));

    editor.addChangeListener((event) => {
      if (event.isUiEvent) {
        return;
      }
      setDirty(
        JSON.stringify(Blockly.serialization.workspaces.save(editor)) !== savedSnapshot.current,
      );
    });

    const resize = new ResizeObserver(() => Blockly.svgResize(editor));
    resize.observe(element);

    return () => {
      resize.disconnect();
      editor.dispose();
      workspace.current = null;
    };
  }, [bot.id, bot.workspace]);

  const saveProgram = useCallback(async () => {
    const editor = workspace.current;
    if (!editor) {
      return;
    }
    setSave({ kind: "saving" });
    const state = Blockly.serialization.workspaces.save(editor);
    try {
      await api.saveProgram(bot.id, state, generateProgram(editor));
      savedSnapshot.current = JSON.stringify(state);
      setDirty(false);
      setSave({ kind: "saved" });
    } catch (error) {
      setSave({
        kind: "error",
        message: error instanceof Error ? error.message : "Could not save",
      });
    }
  }, [bot.id]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "s") {
        event.preventDefault();
        void saveProgram();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [saveProgram]);

  useEffect(() => {
    if (!dirty) {
      return;
    }
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);

  return (
    <div className="editor">
      <div className="editor-bar">
        <span className="muted">
          Drag triggers from the left, then stack actions inside them. Messages accept {"{user}"},{" "}
          {"{channel}"} and {"{option.NAME}"}.
        </span>
        <span className="editor-status">
          {save.kind === "error" && <span className="error">{save.message}</span>}
          {save.kind === "saved" && !dirty && <span className="success">Saved and live</span>}
          {dirty && save.kind !== "saving" && <span className="muted">Unsaved changes</span>}
          <button
            type="button"
            className="button primary"
            onClick={saveProgram}
            disabled={save.kind === "saving" || !dirty}
          >
            {save.kind === "saving" ? "Saving…" : "Save"}
          </button>
        </span>
      </div>
      <div className="editor-canvas" ref={container} />
    </div>
  );
}
