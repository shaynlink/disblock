import * as Blockly from "blockly";
import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Loader2, Save } from "lucide-react";
import { useBlocker } from "react-router";
import { api } from "../api.ts";
import { createTheme, registerBlocks, starterWorkspace, toolbox } from "../blocks/blocks.ts";
import { GenerateError, generateProgram } from "../blocks/generate.ts";
import type { BotDetail, Program } from "../types.ts";
import { Button } from "./ui.tsx";

type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "error"; message: string };

function hasBlocks(workspace: object): boolean {
  return "blocks" in workspace;
}

export function Editor({
  bot,
  onSaved,
}: {
  bot: BotDetail;
  onSaved: (workspace: object, program: Program) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const workspace = useRef<Blockly.WorkspaceSvg | null>(null);
  const savedSnapshot = useRef("");
  const [dirty, setDirty] = useState(false);
  const [save, setSave] = useState<SaveState>({ kind: "idle" });

  // The workspace is loaded once per bot: later saves update `bot.workspace`
  // through `onSaved`, and must not reload the editor under the user.
  const initialWorkspace = useRef(bot.workspace);

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
      grid: { spacing: 24, length: 2, colour: "#e5e5e5", snap: true },
      zoom: { controls: true, wheel: true, startScale: 0.85, maxScale: 2, minScale: 0.4 },
      move: { scrollbars: true, drag: true, wheel: false },
    });
    workspace.current = editor;

    const saved = initialWorkspace.current;
    Blockly.serialization.workspaces.load(hasBlocks(saved) ? saved : starterWorkspace, editor);
    // The starter template counts as unsaved, so the first save registers it.
    savedSnapshot.current = hasBlocks(saved)
      ? JSON.stringify(Blockly.serialization.workspaces.save(editor))
      : "";
    setDirty(!hasBlocks(saved));

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
  }, [bot.id]);

  const saveProgram = useCallback(async () => {
    const editor = workspace.current;
    if (!editor) {
      return;
    }
    for (const block of editor.getAllBlocks(false)) {
      block.setWarningText(null);
    }
    let program: Program;
    try {
      program = generateProgram(editor);
    } catch (error) {
      if (!(error instanceof GenerateError)) {
        throw error;
      }
      // Point at the incomplete block instead of saving.
      const block = editor.getBlockById(error.blockId);
      if (block) {
        block.setWarningText(error.message);
        Blockly.common.setSelected(block);
        editor.centerOnBlock(block.id);
      }
      setSave({ kind: "error", message: error.message });
      return;
    }

    setSave({ kind: "saving" });
    const state = Blockly.serialization.workspaces.save(editor);
    try {
      await api.saveProgram(bot.id, state, program);
      savedSnapshot.current = JSON.stringify(state);
      setDirty(false);
      setSave({ kind: "saved" });
      onSaved(state, program);
    } catch (error) {
      setSave({
        kind: "error",
        message: error instanceof Error ? error.message : "Could not save",
      });
    }
  }, [bot.id, onSaved]);

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

  // Leaving with unsaved blocks: ask first, in the app and when closing the tab.
  const blocker = useBlocker(dirty);
  useEffect(() => {
    if (blocker.state !== "blocked") {
      return;
    }
    if (window.confirm("Your blocks have unsaved changes. Leave without saving?")) {
      blocker.proceed();
    } else {
      blocker.reset();
    }
  }, [blocker]);

  useEffect(() => {
    if (!dirty) {
      return;
    }
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);

  return (
    <div className="flex min-h-[560px] flex-1 flex-col overflow-hidden rounded-xl border bg-card shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-4 border-b px-4 py-3">
        <div>
          <h3 className="font-semibold">Block editor</h3>
          <p className="text-sm text-muted-foreground">
            Drag a trigger from the left, then stack actions inside it. Messages accept{" "}
            <code className="text-xs">{"{user}"}</code>,{" "}
            <code className="text-xs">{"{option.NAME}"}</code> and saved variables like{" "}
            <code className="text-xs">{"{member:points}"}</code>.
          </p>
        </div>
        <div className="flex items-center gap-3">
          {save.kind === "error" && (
            <span className="text-sm text-destructive">{save.message}</span>
          )}
          {save.kind === "saved" && !dirty && (
            <span className="flex items-center gap-1 text-sm text-green-600">
              <Check className="size-4" />
              Saved and live
            </span>
          )}
          {dirty && save.kind !== "saving" && (
            <span className="text-sm text-muted-foreground">Unsaved changes</span>
          )}
          <Button onClick={saveProgram} disabled={save.kind === "saving" || !dirty}>
            {save.kind === "saving" ? <Loader2 className="animate-spin" /> : <Save />}
            {save.kind === "saving" ? "Saving…" : "Save"}
          </Button>
        </div>
      </div>
      <div className="min-h-0 flex-1" ref={container} />
    </div>
  );
}
