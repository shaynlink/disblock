import { Blocks } from "lucide-react";

export function Logo({ subtitle = "Visual Bot Builder" }: { subtitle?: string }) {
  return (
    <span className="flex items-center gap-3">
      <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary">
        <Blocks className="size-5 text-primary-foreground" aria-hidden="true" />
      </span>
      <span className="text-left">
        <span className="block text-xl leading-tight font-bold">Disblock</span>
        <span className="block text-xs text-muted-foreground">{subtitle}</span>
      </span>
    </span>
  );
}
