/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

// Entry point of the VSCode extension.

import { ExtensionContext, workspace, commands } from "vscode"
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient"

let client: LanguageClient

const getLspBin = (context: ExtensionContext) => {
  const config = workspace.getConfiguration("hsp3-analyzer-mini")

  const relativePath = process.env.HSP3_ANALYZER_MINI_LSP_BIN
    || config.get("lsp-bin") as string | undefined
    || "./out/ham-lsp-server-exe"

  return context.asAbsolutePath(relativePath)
}

const getHsp3Home = () => {
  // 現在の最新版の既定のインストールディレクトリ
  const DEFAULT_DIR = "C:/Program Files (x86)/hsp351"

  const config = workspace.getConfiguration("hsp3-analyzer-mini")
  return config.get<string>("hsp3-home")
    || process.env.HSP3_HOME
    || config.get<string>("hsp3-root")
    || process.env.HSP3_ROOT
    || DEFAULT_DIR
}

const startLspClient = (context: ExtensionContext) => {
  const lspFullPath = getLspBin(context)
  const hsp3Home = getHsp3Home()

  let serverOptions: ServerOptions = {
    command: lspFullPath,
    args: ["--hsp", hsp3Home, "lsp"],
  }

  let clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "hsp3" },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/.clientrc"),
    },
  }

  // Start language server and client.
  client = new LanguageClient(
    "hsp3",
    "hsp3 LSP",
    serverOptions,
    clientOptions
  )
  client.start()
}

export function activate(context: ExtensionContext) {
  commands.registerCommand("getLspBin", () => getLspBin(context))

  startLspClient(context)
}

export function deactivate(): Thenable<void> | undefined {
  if (client) {
    return client.stop()
  }
}
