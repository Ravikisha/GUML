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

  /**
   * Run one of the server's document-level source actions.
   *
   * Both are ordinary code actions, so this asks the server for them by kind rather than
   * reimplementing anything: the compiler decides what the edit is, and the extension only applies it.
   * A reimplementation here would be a second opinion about GUML, which is the one thing this whole
   * architecture is arranged to prevent.
   */
  const runSourceAction = async (kind: string, label: string): Promise<void> => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "guml") {
      void vscode.window.showInformationMessage("Open a `.guml` file first.");
      return;
    }
    const whole = new vscode.Range(
      0,
      0,
      editor.document.lineCount,
      editor.document.lineAt(Math.max(0, editor.document.lineCount - 1)).text.length,
    );
    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      "vscode.executeCodeActionProvider",
      editor.document.uri,
      whole,
      kind,
    );
    const action = actions?.find((a) => a.kind?.value === kind && a.edit);
    if (!action?.edit) {
      void vscode.window.showInformationMessage(`Nothing to ${label.toLowerCase()}.`);
      return;
    }
    await vscode.workspace.applyEdit(action.edit);
  };

  context.subscriptions.push(
    vscode.commands.registerCommand("guml.fixAll", () =>
      runSourceAction("source.fixAll", "fix"),
    ),
    // `source` rather than `source.fixAll`: repair also *deletes* — a code fence, trailing prose — so it
    // must not be reachable by anything a user configured to run on save under the name "fix".
    vscode.commands.registerCommand("guml.repair", () => runSourceAction("source", "repair")),
    vscode.commands.registerCommand("guml.restartServer", async () => {
      await client?.restart();
    }),
  );

  if (vscode.workspace.getConfiguration("guml").get<boolean>("fixAllOnSave")) {
    context.subscriptions.push(
      vscode.workspace.onWillSaveTextDocument((event) => {
        if (event.document.languageId !== "guml") return;
        // Ordered after the formatter's own `onWillSave` above, which is what we want: format first so
        // spans are where the compiler expects them, then apply the fixes.
        event.waitUntil(runSourceAction("source.fixAll", "fix").then(() => []));
      }),
    );
  }

  context.subscriptions.push({ dispose: () => void client?.stop() });
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}
