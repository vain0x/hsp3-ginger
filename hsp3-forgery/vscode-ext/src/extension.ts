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
    const config = workspace.getConfiguration("hsp3-forgery")
    const ext = ".exe"

    const relativePath = process.env.HSP3_FORGERY_LSP_BIN
        || config.get("lsp-bin") as string | undefined
        || "./out/hsp3-forgery-lsp" + ext

    return context.asAbsolutePath(relativePath)
}

const getHspRoot = () => {
    // 現在の最新版の既定のインストールディレクトリ
    const DEFAULT_DIR = "C:/Program Files (x86)/hsp351"

    const config = workspace.getConfiguration("hsp3-forgery")
    return config.get("hsp3-root") as string | undefined
        || process.env.HSP3_ROOT
        || DEFAULT_DIR
}

const startLspClient = (context: ExtensionContext) => {
    const lspFullPath = getLspBin(context)
    const hspRoot = getHspRoot()

    let serverOptions: ServerOptions = {
        command: lspFullPath,
        args: ["--hsp3", hspRoot],
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
        "HSP3 フォージェリ LSP",
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
