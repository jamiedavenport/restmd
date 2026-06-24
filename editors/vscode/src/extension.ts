import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(_context: ExtensionContext): void {
  // RESTMD_LSP_PATH (set by the dev launch config) wins, then the setting, then PATH.
  const command =
    process.env.RESTMD_LSP_PATH ||
    workspace.getConfiguration("restmd").get<string>("serverPath") ||
    "restmd-lsp";

  const serverOptions: ServerOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

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
