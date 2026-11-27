import * as Blockly from "blockly";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  Check,
  ExternalLink,
  LayoutTemplate,
  Loader2,
  RefreshCw,
  Save,
} from "lucide-react";
import { useBlocker, useSearchParams } from "react-router";
import { ApiError, api } from "../api.ts";
import { createTheme, registerBlocks, starterWorkspace, toolbox } from "../blocks/blocks.ts";
import { setDiscordPickerResources } from "../blocks/discordPicker.ts";
import { type BotTemplate, hasCustomWorkspaceBlocks, insertTemplate } from "../blocks/templates.ts";
import {
  GenerateError,
  type MissingIntent,
  findMissingIntents,
  generateProgram,
  generateProgramWithMap,
  resolveErrorBlockId,
} from "../blocks/generate.ts";
import { buttonClass } from "../lib/styles.ts";
import type { BotDetail, BotIntents, BotResources, Program } from "../types.ts";
import { DiscordPreview } from "./DiscordPreview.tsx";
import { TemplateDialog } from "./TemplateDialog.tsx";
import { Button } from "./ui.tsx";

type SaveState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "error"; message: string };

type IntentsState =
  | { kind: "loading" }
  | { kind: "ready"; intents: BotIntents }
  | { kind: "error" };

type PreviewState = { program: Program | null; error: string | null };

const INTENT_WARNING_ID = "disblock_intent";
const SAVE_WARNING_ID = "disblock_save";

function hasBlocks(workspace: object): boolean {
  return "blocks" in workspace;
}

function focusWarningBlock(editor: Blockly.WorkspaceSvg, block: Blockly.BlockSvg) {
  Blockly.common.setSelected(block);
  editor.centerOnBlock(block.id);
  const icon = block.getIcon(Blockly.icons.IconType.WARNING);
  if (icon instanceof Blockly.icons.WarningIcon) {
    void icon.setBubbleVisible(true);
  }
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
  const intentWarningBlocks = useRef<Set<string>>(new Set());
  const saveWarningBlock = useRef<string | null>(null);
  const latestIntents = useRef<IntentsState>({ kind: "loading" });

  const [dirty, setDirty] = useState(false);
  const [save, setSave] = useState<SaveState>({ kind: "idle" });
  const [intentsState, setIntentsState] = useState<IntentsState>({ kind: "loading" });
  const [resources, setResources] = useState<BotResources | null>(null);
  const [refreshingResources, setRefreshingResources] = useState(false);
  const [missingIntents, setMissingIntents] = useState<MissingIntent[]>([]);
  const [checkingIntents, setCheckingIntents] = useState(false);
  const [preview, setPreview] = useState<PreviewState>({ program: null, error: null });
  const [searchParams, setSearchParams] = useSearchParams();
  const [templatesOpen, setTemplatesOpen] = useState(() => searchParams.get("templates") === "1");
  const [hasCustomBlocks, setHasCustomBlocks] = useState(false);

  // The workspace is loaded once per bot: later saves update `bot.workspace`
  // through `onSaved`, and must not reload the editor under the user.
  const initialWorkspace = useRef(bot.workspace);

  const syncIntentWarnings = useCallback(
    (editor: Blockly.WorkspaceSvg, state: IntentsState): MissingIntent[] => {
      for (const id of intentWarningBlocks.current) {
        editor.getBlockById(id)?.setWarningText(null, INTENT_WARNING_ID);
      }
      intentWarningBlocks.current.clear();

      if (state.kind !== "ready") {
        setMissingIntents([]);
        return [];
      }
      const missing = findMissingIntents(editor, state.intents);
      for (const item of missing) {
        editor.getBlockById(item.blockId)?.setWarningText(item.message, INTENT_WARNING_ID);
        intentWarningBlocks.current.add(item.blockId);
      }
      setMissingIntents(missing);
      return missing;
    },
    [],
  );

  const clearSaveWarning = useCallback((editor: Blockly.WorkspaceSvg) => {
    if (saveWarningBlock.current) {
      editor.getBlockById(saveWarningBlock.current)?.setWarningText(null, SAVE_WARNING_ID);
      saveWarningBlock.current = null;
    }
  }, []);

  const syncPreview = useCallback((editor: Blockly.WorkspaceSvg) => {
    try {
      setPreview({ program: generateProgram(editor), error: null });
    } catch (error) {
      setPreview({
        program: null,
        error: error instanceof Error ? error.message : "Could not generate preview.",
      });
    }
  }, []);

  const showSaveError = useCallback(
    (editor: Blockly.WorkspaceSvg, blockId: string | null, message: string) => {
      clearSaveWarning(editor);
      const block = blockId ? editor.getBlockById(blockId) : null;
      if (block) {
        block.setWarningText(message, SAVE_WARNING_ID);
        saveWarningBlock.current = block.id;
        focusWarningBlock(editor, block);
      }
      setSave({ kind: "error", message });
    },
    [clearSaveWarning],
  );

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
    setHasCustomBlocks(hasCustomWorkspaceBlocks(editor));
    syncPreview(editor);

    editor.addChangeListener((event) => {
      if (event.isUiEvent) {
        return;
      }
      setDirty(
        JSON.stringify(Blockly.serialization.workspaces.save(editor)) !== savedSnapshot.current,
      );
      setHasCustomBlocks(hasCustomWorkspaceBlocks(editor));
      if (
        event.type === Blockly.Events.BLOCK_CHANGE ||
        event.type === Blockly.Events.BLOCK_MOVE ||
        event.type === Blockly.Events.BLOCK_DELETE ||
        event.type === Blockly.Events.BLOCK_CREATE
      ) {
        clearSaveWarning(editor);
        setSave((current) => (current.kind === "error" ? { kind: "idle" } : current));
      }
      syncIntentWarnings(editor, latestIntents.current);
      syncPreview(editor);
    });

    const resize = new ResizeObserver(() => Blockly.svgResize(editor));
    resize.observe(element);

    let active = true;
    latestIntents.current = { kind: "loading" };
    setIntentsState({ kind: "loading" });
    api
      .botIntents(bot.id)
      .then((intents) => {
        if (!active) {
          return;
        }
        const next: IntentsState = { kind: "ready", intents };
        latestIntents.current = next;
        setIntentsState(next);
        syncIntentWarnings(editor, next);
      })
      .catch(() => {
        if (!active) {
          return;
        }
        const next: IntentsState = { kind: "error" };
        latestIntents.current = next;
        setIntentsState(next);
        syncIntentWarnings(editor, next);
      });

    api
      .botResources(bot.id)
      .then((loadedResources) => {
        if (!active) {
          return;
        }
        setResources(loadedResources);
        setDiscordPickerResources(loadedResources);
        editor.render();
      })
      .catch(() => {
        if (!active) {
          return;
        }
        setResources(null);
        setDiscordPickerResources(null);
      });

    const trackedIntentWarnings = intentWarningBlocks.current;

    return () => {
      active = false;
      resize.disconnect();
      editor.dispose();
      workspace.current = null;
      setDiscordPickerResources(null);
      trackedIntentWarnings.clear();
      saveWarningBlock.current = null;
    };
  }, [bot.id, clearSaveWarning, syncIntentWarnings, syncPreview]);

  const refreshResources = useCallback(async () => {
    setRefreshingResources(true);
    try {
      const loaded = await api.botResources(bot.id);
      setResources(loaded);
      setDiscordPickerResources(loaded);
      workspace.current?.render();
    } catch {
      // Keep current resources if Discord is temporarily unreachable.
    } finally {
      setRefreshingResources(false);
    }
  }, [bot.id]);

  const recheckIntents = useCallback(async () => {
    const editor = workspace.current;
    if (!editor) {
      return;
    }
    setCheckingIntents(true);
    try {
      const intents = await api.botIntents(bot.id);
      const next: IntentsState = { kind: "ready", intents };
      latestIntents.current = next;
      setIntentsState(next);
      const missing = syncIntentWarnings(editor, next);
      if (missing.length === 0) {
        setSave((current) => (current.kind === "error" ? { kind: "idle" } : current));
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Could not check intents";
      setSave({ kind: "error", message });
    } finally {
      setCheckingIntents(false);
    }
  }, [bot.id, syncIntentWarnings]);

  const saveProgram = useCallback(async () => {
    const editor = workspace.current;
    if (!editor) {
      return;
    }
    clearSaveWarning(editor);

    let program: Program;
    let map: ReturnType<typeof generateProgramWithMap>["map"];
    try {
      ({ program, map } = generateProgramWithMap(editor));
    } catch (error) {
      if (!(error instanceof GenerateError)) {
        throw error;
      }
      showSaveError(editor, error.blockId, error.message);
      return;
    }

    const needsPrivilegedIntent = program.handlers.some(
      (handler) =>
        handler.trigger.type === "member_join" ||
        handler.trigger.type === "member_leave" ||
        handler.trigger.type === "member_update" ||
        handler.trigger.type === "message_contains" ||
        handler.trigger.type === "message_edit",
    );
    if (needsPrivilegedIntent) {
      setSave({ kind: "checking" });
      try {
        const freshIntents = await api.botIntents(bot.id);
        const next: IntentsState = { kind: "ready", intents: freshIntents };
        latestIntents.current = next;
        setIntentsState(next);
        const missing = syncIntentWarnings(editor, next);
        if (missing.length > 0) {
          const first = missing[0];
          const block = editor.getBlockById(first.blockId);
          if (block) {
            focusWarningBlock(editor, block);
          }
          setSave({ kind: "error", message: first.message });
          return;
        }
      } catch (error) {
        if (error instanceof ApiError && error.status === 400) {
          setSave({ kind: "error", message: error.message });
          return;
        }
      }
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
      const message = error instanceof Error ? error.message : "Could not save";
      if (error instanceof ApiError && error.location) {
        const blockId = resolveErrorBlockId(map, error.location);
        showSaveError(editor, blockId, message);
      } else {
        setSave({ kind: "error", message });
      }
    }
  }, [bot.id, clearSaveWarning, onSaved, showSaveError, syncIntentWarnings]);

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

  const uniqueMissingIntents = useMemo(() => {
    const seen = new Set<MissingIntent["intent"]>();
    return missingIntents.filter((item) => {
      if (seen.has(item.intent)) {
        return false;
      }
      seen.add(item.intent);
      return true;
    });
  }, [missingIntents]);

  const applyTemplate = useCallback(
    (template: BotTemplate, values: Record<string, string>, mode: "append" | "replace") => {
      const editor = workspace.current;
      if (!editor) {
        return;
      }
      clearSaveWarning(editor);
      setSave({ kind: "idle" });
      const firstBlock = insertTemplate(editor, template, values, mode);
      Blockly.svgResize(editor);
      if (firstBlock) {
        editor.centerOnBlock(firstBlock.id);
      }
      syncIntentWarnings(editor, latestIntents.current);
      syncPreview(editor);
      setHasCustomBlocks(hasCustomWorkspaceBlocks(editor));
      setDirty(
        JSON.stringify(Blockly.serialization.workspaces.save(editor)) !== savedSnapshot.current,
      );
    },
    [clearSaveWarning, syncIntentWarnings, syncPreview],
  );

  const openTemplates = () => {
    const editor = workspace.current;
    if (editor) {
      setHasCustomBlocks(hasCustomWorkspaceBlocks(editor));
    }
    setTemplatesOpen(true);
  };

  const closeTemplates = () => {
    setTemplatesOpen(false);
    if (searchParams.has("templates")) {
      const next = new URLSearchParams(searchParams);
      next.delete("templates");
      setSearchParams(next, { replace: true });
    }
  };

  const busy = save.kind === "checking" || save.kind === "saving";

  return (
    <div className="flex min-h-[560px] flex-1 flex-col overflow-hidden rounded-xl border bg-card shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-4 border-b px-4 py-3">
        <div>
          <h3 className="font-semibold">Block editor</h3>
          <p className="text-sm text-muted-foreground">
            Drag a trigger from the left, or start from a template. Messages accept{" "}
            <code className="text-xs">{"{user}"}</code>,{" "}
            <code className="text-xs">{"{option.NAME}"}</code>,{" "}
            <code className="text-xs">{"{selected}"}</code> and saved variables like{" "}
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
          {dirty && !busy && <span className="text-sm text-muted-foreground">Unsaved changes</span>}
          <Button
            variant="outline"
            onClick={refreshResources}
            disabled={busy || refreshingResources}
            title="Reload Discord servers, channels, and roles"
          >
            <RefreshCw className={refreshingResources ? "animate-spin" : undefined} />
            {resources
              ? `${resources.guilds.length} server${resources.guilds.length === 1 ? "" : "s"}`
              : "Sync Discord"}
          </Button>
          <Button variant="outline" onClick={openTemplates} disabled={busy}>
            <LayoutTemplate />
            Templates
          </Button>
          <Button onClick={saveProgram} disabled={busy || !dirty}>
            {busy ? <Loader2 className="animate-spin" /> : <Save />}
            {save.kind === "checking"
              ? "Checking intents…"
              : save.kind === "saving"
                ? "Saving…"
                : "Save"}
          </Button>
        </div>
      </div>
      {intentsState.kind === "ready" && uniqueMissingIntents.length > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-amber-500/30 bg-amber-500/10 px-4 py-2.5 text-sm">
          <div className="flex items-start gap-2 text-amber-950">
            <AlertTriangle className="mt-0.5 size-4 shrink-0 text-amber-600" />
            <p>
              <span className="font-medium">
                Missing Discord {uniqueMissingIntents.length === 1 ? "intent" : "intents"}:
              </span>{" "}
              {uniqueMissingIntents.map((item) => item.label).join(" and ")}. Enable{" "}
              {uniqueMissingIntents.length === 1 ? "it" : "them"} under{" "}
              <span className="font-medium">Bot → Privileged Gateway Intents</span> before saving.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <a
              href={`https://discord.com/developers/applications/${bot.application_id}/bot`}
              target="_blank"
              rel="noreferrer"
              className={buttonClass("outline", "sm")}
            >
              <ExternalLink />
              Open Discord intents
            </a>
            <Button variant="outline" size="sm" onClick={recheckIntents} disabled={checkingIntents}>
              <RefreshCw className={checkingIntents ? "animate-spin" : undefined} />
              Re-check
            </Button>
          </div>
        </div>
      )}
      <div className="flex min-h-0 flex-1 flex-col lg:flex-row">
        <div className="min-h-[560px] min-w-0 flex-1" ref={container} />
        <DiscordPreview bot={bot} program={preview.program} error={preview.error} />
      </div>
      <TemplateDialog
        open={templatesOpen}
        onClose={closeTemplates}
        onApply={applyTemplate}
        hasCustomBlocks={hasCustomBlocks}
        intents={intentsState.kind === "ready" ? intentsState.intents : null}
        resources={resources}
      />
    </div>
  );
}
