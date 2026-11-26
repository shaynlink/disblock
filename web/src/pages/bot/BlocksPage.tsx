import { Editor } from "../../components/Editor.tsx";
import { useBot } from "./context.ts";

export function BlocksPage() {
  const { detail, onProgramSaved } = useBot();
  return <Editor bot={detail} onSaved={onProgramSaved} />;
}
