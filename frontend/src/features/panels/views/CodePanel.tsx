import { CodeEditor } from "@features/editor/views";
import { Panel } from "@constants";

export interface CodePanelProps {
  id: string;
}

export function CodePanel(props: CodePanelProps) {
  let file = props.id;

  if (file.startsWith(Panel.FilePrefix)) {
    file = file.slice(Panel.FilePrefix.length);
  }

  return <CodeEditor file={file} />;
}
