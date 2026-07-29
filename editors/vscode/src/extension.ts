import { existsSync } from "node:fs";
import { join } from "node:path";
import * as vscode from "vscode";
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

/**
 * The extension is a client and nothing more.
 *
 * Every answer — diagnostics, highlighting, completion, formatting — comes from `guml-lsp`, which
 * in turn calls the compiler. That is the point: a human sees exactly the errors a model sees,
 * because they are literally the same function call. An extension that re-implemented any of it
 * would become a second opinion about GUML, and the one thing this project cannot afford is two
 * answers to the same question.
 */

let client: LanguageClient | undefined;

/**
 * Where the server binary is.
 *
 * Configured path wins; otherwise the workspace's own `cargo build` output, since anyone editing
 * `.guml` files in this repository has one. `PATH` last, for an installed copy.
 */
function serverPath(): string | undefined {
  const configured = vscode.workspace.getConfiguration("guml").get<string>("serverPath");
  if (configured) return configured;

  const exe = process.platform === "win32" ? "guml-lsp.exe" : "guml-lsp";
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    for (const profile of ["release", "debug"]) {
      const candidate = join(folder.uri.fsPath, "target", profile, exe);
      if (existsSync(candidate)) return candidate;
    }
  }
  // Let the OS resolve it; `start` will report a clear failure if it cannot.
  return exe;
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const command = serverPath();
  if (!command) return;

  const serverOptions: ServerOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "guml" }],
    // The registry and the spec are compiler-side, so a change to either only matters when the
    // server restarts. Nothing in the workspace needs watching.
    synchronize: {},
  };

  client = new LanguageClient("guml", "GUML", serverOptions, clientOptions);

  try {
    await client.start();
  } catch (error) {
    // A missing binary is the overwhelmingly likely cause, and the fix is one command.
    void vscode.window.showWarningMessage(
      `GUML language server did not start (${String(error)}). Build it with ` +
        "`cargo build -p guml-lsp --release`, or set `guml.serverPath`.",
    );
    return;
  }

  if (vscode.workspace.getConfiguration("guml").get<boolean>("formatOnSave")) {
    context.subscriptions.push(
      vscode.workspace.onWillSaveTextDocument((event) => {
        if (event.document.languageId !== "guml") return;
        // `waitUntil` so the edit lands before the file is written. The formatter is a whole-
        // document replace, which is why this is one promise rather than a stream of edits.
        event.waitUntil(
          vscode.commands
            .executeCommand<vscode.TextEdit[]>(
              "vscode.executeFormatDocumentProvider",
              event.document.uri,
              { tabSize: 2, insertSpaces: true },
            )
            .then((edits) => edits ?? []),
        );
      }),
    );
  }

  context.subscriptions.push({ dispose: () => void client?.stop() });
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}
