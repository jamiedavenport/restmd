import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(_context: ExtensionContext): void {
  // The language server ships inside the `restmd` CLI and runs as `restmd lsp`.
  // RESTMD_PATH (set by the dev launch config) wins, then the setting, then PATH.
  const command =
    process.env.RESTMD_PATH ||
    workspace.getConfiguration("restmd").get<string>("serverPath") ||
    "restmd";

  const run = { command, args: ["lsp"], transport: TransportKind.stdio };
  const serverOptions: ServerOptions = { run, debug: run };

  // Engage only inside `.restmd` directories — files render as plain markdown
  // everywhere else.
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "markdown", pattern: "**/.restmd/**" }],
  };

  client = new LanguageClient("restmd", "restmd", serverOptions, clientOptions);
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
