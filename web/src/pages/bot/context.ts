import { useOutletContext } from "react-router";
import type { Bot, BotDetail, Program } from "../../types.ts";

/** What a bot's pages share, provided by `BotLayout`. */
export interface BotOutlet {
  /** Latest summary, refreshed every few seconds for the live status. */
  bot: Bot;
  /** Editor state and program, as last loaded or saved. */
  detail: BotDetail;
  setEnabled: (enabled: boolean) => Promise<void>;
  onProgramSaved: (workspace: object, program: Program) => void;
}

export function useBot(): BotOutlet {
  return useOutletContext<BotOutlet>();
}
