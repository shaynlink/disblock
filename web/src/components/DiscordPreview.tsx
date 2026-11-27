import {
  ChevronDown,
  Eye,
  FileText,
  Image as ImageIcon,
  Lock,
  MessageSquare,
  Vote,
} from "lucide-react";
import { avatarUrl } from "../api.ts";
import { getPreviewMessages, previewText, type PreviewMessage } from "../blocks/preview.ts";
import { cn } from "../lib/styles.ts";
import type {
  AttachmentSpec,
  Bot as BotRecord,
  ButtonSpec,
  ComponentRow,
  Embed,
  ModalInputSpec,
  PollSpec,
  Program,
} from "../types.ts";

const BUTTON_STYLES: Record<Extract<ButtonSpec, { type: "action" }>["style"], string> = {
  primary: "bg-[#5865f2] hover:bg-[#4752c4]",
  secondary: "bg-[#4e5058] hover:bg-[#6d6f78]",
  success: "bg-[#248046] hover:bg-[#1a6334]",
  danger: "bg-[#da373c] hover:bg-[#a12828]",
};

function safeImageUrl(value: string | undefined): string | undefined {
  if (!value) {
    return undefined;
  }
  try {
    const url = new URL(value);
    return url.protocol === "https:" || url.protocol === "http:" ? url.toString() : undefined;
  } catch {
    return undefined;
  }
}

function embedColour(value: string | undefined): string | undefined {
  return value && /^#[0-9a-f]{6}$/i.test(value) ? value : undefined;
}

function EmbedPreview({ embed }: { embed: Embed }) {
  const thumbnailUrl = safeImageUrl(embed.thumbnail_url);
  const imageUrl = safeImageUrl(embed.image_url);

  return (
    <div
      className="mt-2.5 max-w-full rounded border-l-4 border-white/20 bg-[#2b2d31] p-3 text-sm text-[#dbdee1]"
      style={embedColour(embed.color) ? { borderLeftColor: embedColour(embed.color) } : undefined}
    >
      {embed.author && (
        <div className="mb-1 text-xs font-semibold text-white">
          {previewText(embed.author.name)}
        </div>
      )}
      <div className="flex gap-3">
        <div className="min-w-0 flex-1">
          {embed.title && <p className="font-semibold text-white">{previewText(embed.title)}</p>}
          {embed.description && (
            <p className="mt-1 whitespace-pre-wrap text-xs leading-relaxed text-[#dbdee1]">
              {previewText(embed.description)}
            </p>
          )}
          {embed.fields.length > 0 && (
            <dl className="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2">
              {embed.fields.map((field, index) => (
                <div
                  key={`${field.name}-${index}`}
                  className={field.inline ? undefined : "sm:col-span-2"}
                >
                  <dt className="text-xs font-semibold text-white">{previewText(field.name)}</dt>
                  <dd className="mt-0.5 whitespace-pre-wrap text-xs text-[#dbdee1]">
                    {previewText(field.value)}
                  </dd>
                </div>
              ))}
            </dl>
          )}
        </div>
        {thumbnailUrl && (
          <img
            src={thumbnailUrl}
            alt="Embed thumbnail"
            className="size-16 shrink-0 rounded object-cover"
            referrerPolicy="no-referrer"
          />
        )}
      </div>
      {imageUrl && (
        <img
          src={imageUrl}
          alt="Embed image"
          className="mt-3 max-h-48 w-full rounded object-cover"
          referrerPolicy="no-referrer"
        />
      )}
      {(embed.footer || embed.timestamp) && (
        <p className="mt-3 text-[11px] text-[#b5bac1]">
          {embed.footer ? previewText(embed.footer.text) : ""}
          {embed.footer && embed.timestamp ? " • " : ""}
          {embed.timestamp ? "Today at 12:00" : ""}
        </p>
      )}
    </div>
  );
}

function ButtonPreview({ button }: { button: ButtonSpec }) {
  const label = [button.emoji, button.label].filter(Boolean).join(" ") || "Button";
  return (
    <span
      aria-label={`${label}, preview only`}
      className={cn(
        "inline-flex min-h-8 items-center rounded px-3 py-1.5 text-xs font-medium text-white",
        button.disabled ? "cursor-not-allowed opacity-40" : "cursor-default",
        button.type === "action" ? BUTTON_STYLES[button.style] : "bg-[#4e5058]",
      )}
    >
      {previewText(label)}
    </span>
  );
}

function entityDefaultPlaceholder(kind: string): string {
  switch (kind) {
    case "user":
      return "Select member(s)";
    case "role":
      return "Select role(s)";
    case "mentionable":
      return "Select member(s) or role(s)";
    case "channel":
      return "Select channel(s)";
    default:
      return "Select an option";
  }
}

function ComponentsPreview({ components }: { components: ComponentRow[] }) {
  return (
    <div className="mt-2 flex flex-col items-start gap-1.5">
      {components.map((row, index) =>
        row.type === "buttons" ? (
          <div key={index} className="flex flex-wrap gap-1.5">
            {row.buttons.map((button, buttonIndex) => (
              <ButtonPreview key={`${button.type}-${buttonIndex}`} button={button} />
            ))}
          </div>
        ) : (
          <div
            key={index}
            aria-label="Select menu preview only"
            className={cn(
              "flex h-9 w-full items-center justify-between rounded border border-black/20 bg-[#1e1f22] px-3 text-xs text-[#b5bac1]",
              row.disabled && "opacity-40",
            )}
          >
            <span>
              {previewText(
                row.placeholder ||
                  (row.type === "entity_select_menu"
                    ? entityDefaultPlaceholder(row.kind)
                    : "Select an option"),
              )}
            </span>
            <ChevronDown className="size-4" aria-hidden="true" />
          </div>
        ),
      )}
    </div>
  );
}

function PollPreview({ poll }: { poll: PollSpec }) {
  return (
    <div className="mt-2.5 rounded-lg border border-white/10 bg-[#2b2d31] p-3">
      <div className="flex items-center gap-1.5 text-xs font-semibold text-white">
        <Vote className="size-3.5 text-[#b5bac1]" aria-hidden="true" />
        <span>{previewText(poll.question || "Poll")}</span>
      </div>
      <div className="mt-2 space-y-1.5">
        {poll.answers.map((answer, idx) => (
          <div
            key={idx}
            className="flex items-center gap-2 rounded border border-white/10 bg-[#1e1f22] px-2.5 py-1.5 text-xs text-[#dbdee1]"
          >
            {answer.emoji && <span>{answer.emoji}</span>}
            <span>{previewText(answer.text)}</span>
          </div>
        ))}
      </div>
      <p className="mt-2 text-[10px] text-[#949ba4]">
        {poll.allow_multiselect ? "Multiple choices allowed" : "Single choice"} •{" "}
        {poll.duration_hours}h
      </p>
    </div>
  );
}

function AttachmentsPreview({ attachments }: { attachments: AttachmentSpec[] }) {
  return (
    <div className="mt-2 space-y-1.5">
      {attachments.map((file, idx) => (
        <div
          key={idx}
          className="flex items-center gap-2 rounded border border-white/10 bg-[#2b2d31] px-3 py-2 text-xs text-[#dbdee1]"
        >
          <FileText className="size-4 shrink-0 text-[#949ba4]" aria-hidden="true" />
          <div className="min-w-0 flex-1">
            <p className="truncate font-medium text-[#00a8fc]">{file.filename || "file.txt"}</p>
            <p className="truncate text-[11px] text-[#949ba4]">{previewText(file.content)}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

function ModalPreview({ modal }: { modal: { title: string; inputs: ModalInputSpec[] } }) {
  return (
    <div className="mt-2 rounded-lg border border-white/15 bg-[#313338] p-3">
      <p className="text-sm font-semibold text-white">{previewText(modal.title || "Modal")}</p>
      <div className="mt-2.5 space-y-2.5">
        {modal.inputs.map((input, idx) => (
          <div key={idx}>
            <p className="text-[11px] font-semibold uppercase tracking-wide text-[#b5bac1]">
              {previewText(input.label || "Field")}
              {input.required && <span className="ml-0.5 text-[#f23f43]">*</span>}
            </p>
            <div
              className={cn(
                "mt-1 rounded border border-black/30 bg-[#1e1f22] px-2.5 py-1.5 text-xs text-[#949ba4]",
                input.style === "paragraph" && "min-h-12",
              )}
            >
              {previewText(input.value || input.placeholder || "User types here…")}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function MessagePreview({ message, bot }: { message: PreviewMessage; bot: BotRecord }) {
  return (
    <article className="border-t border-white/10 px-4 py-4 first:border-t-0">
      <div className="mb-2 flex flex-wrap items-center gap-1.5 text-[11px] font-medium text-[#b5bac1]">
        <span className="rounded bg-white/10 px-1.5 py-0.5">{message.kind}</span>
        <span>when {message.trigger}</span>
        {message.condition && <span>• {message.condition}</span>}
        {message.ephemeral && (
          <span className="inline-flex items-center gap-1 rounded bg-[#5865f2]/25 px-1.5 py-0.5 text-[#c9d0ff]">
            <Lock className="size-3" aria-hidden="true" />
            Only you
          </span>
        )}
      </div>
      <div className="flex items-start gap-2.5">
        <img
          src={avatarUrl(bot.application_id, bot.avatar)}
          alt=""
          className="mt-0.5 size-9 shrink-0 rounded-full bg-[#5865f2]"
          referrerPolicy="no-referrer"
        />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="text-sm font-semibold text-white">{bot.name}</span>
            <span className="rounded bg-[#5865f2] px-1 py-0.5 text-[9px] font-bold tracking-wide text-white uppercase">
              App
            </span>
          </div>
          {message.content && (
            <p className="mt-1 whitespace-pre-wrap break-words text-sm leading-snug text-[#dbdee1]">
              {previewText(message.content)}
            </p>
          )}
          {message.embeds?.map((embed, index) => (
            <EmbedPreview key={index} embed={embed} />
          ))}
          {message.poll && <PollPreview poll={message.poll} />}
          {message.attachments && message.attachments.length > 0 && (
            <AttachmentsPreview attachments={message.attachments} />
          )}
          {message.components && <ComponentsPreview components={message.components} />}
          {message.modal && <ModalPreview modal={message.modal} />}
        </div>
      </div>
    </article>
  );
}

export function DiscordPreview({
  bot,
  program,
  error,
}: {
  bot: BotRecord;
  program: Program | null;
  error: string | null;
}) {
  const messages = program ? getPreviewMessages(program) : [];

  return (
    <aside className="flex min-h-[260px] w-full flex-col border-t bg-[#1e1f22] text-[#f2f3f5] lg:min-h-0 lg:w-[380px] lg:border-t-0 lg:border-l">
      <header className="flex items-start justify-between gap-3 border-b border-white/10 px-4 py-3">
        <div>
          <h4 className="flex items-center gap-2 text-sm font-semibold">
            <Eye className="size-4 text-[#b5bac1]" aria-hidden="true" />
            Live Discord preview
          </h4>
          <p className="mt-0.5 text-xs text-[#b5bac1]">
            Sample values replace dynamic placeholders.
          </p>
        </div>
        {program && (
          <span className="text-xs text-[#b5bac1]">
            {messages.length} message{messages.length === 1 ? "" : "s"}
          </span>
        )}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {error ? (
          <div
            className="m-4 rounded-lg border border-amber-400/30 bg-amber-400/10 p-3 text-sm text-amber-100"
            role="status"
          >
            Finish the incomplete connected block to refresh the preview. {error}
          </div>
        ) : messages.length === 0 ? (
          <div className="flex min-h-48 flex-col items-center justify-center px-6 text-center text-sm text-[#b5bac1]">
            <MessageSquare className="mb-3 size-7" aria-hidden="true" />
            Add a reply, message, modal, or direct-message block to see it here.
          </div>
        ) : (
          messages.map((message, index) => (
            <MessagePreview key={index} message={message} bot={bot} />
          ))
        )}
      </div>
      <p className="flex items-center gap-2 border-t border-white/10 px-4 py-2.5 text-[11px] text-[#949ba4]">
        <ImageIcon className="size-3.5" aria-hidden="true" />
        Preview only — buttons, menus, and modals are not interactive.
      </p>
    </aside>
  );
}
