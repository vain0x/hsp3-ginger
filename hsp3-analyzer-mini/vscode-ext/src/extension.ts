/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

// Entry point of the VSCode extension.

import * as fs from "fs/promises"
import { watch, FSWatcher } from "fs"
import { ExtensionContext, workspace, window, CancellationTokenSource } from "vscode"
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient"

/** 開発モード */
const DEV = process.env["HSP3_ANALYZER_MINI_DEV"] === "1"

// -----------------------------------------------
// 設定の読み込みなど
// -----------------------------------------------

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

const lintIsEnabled = () =>
  workspace.getConfiguration("hsp3-analyzer-mini").get<boolean>("lint-enabled") ?? true

// -----------------------------------------------
// LSPクライアント
// -----------------------------------------------

const newLspClient = (lspBin: string): LanguageClient => {
  const hsp3Home = getHsp3Home()
  const lintEnabled = lintIsEnabled()

  let serverOptions: ServerOptions = {
    command: lspBin,
    args: ["--hsp", hsp3Home, "lsp"],
    options: {
      env: {
        "HAM_LINT": lintEnabled ? "1" : "",
      }
    }
  }

  let clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "hsp3" },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/.clientrc"),
    },
  }

  return new LanguageClient("hsp3", "hsp3 LSP", serverOptions, clientOptions)
}

// -----------------------------------------------
// 開発者モード
// -----------------------------------------------

const dev = (context: ExtensionContext): void => {
  console.log("ham: 開発者モードです。")

  const THROTTLE = 500

  // ログ出力 (管理者ツールのコンソール)
  const log = (msg: string, ...args: unknown[]) => {
    console.log("ham:", msg, ...args)
  }

  // エラー表示
  let initialError = true
  const error = (err: unknown) => {
    console.error("err:", err)
    if (initialError) {
      initialError = false
      window.showErrorMessage("ham: Error." + (err as any)?.message)
    }
  }

  // LSPクライアントを自動で再読み込みする。
  const lspBin = getLspBin(context)
  const lspBackupBin = lspBin.replace(/\.exe$/, "") + "_orig.exe"

  const client = newLspClient(lspBackupBin)
  context.subscriptions.push({ dispose: () => client.stop() })

  const waitClientStateChange = () => new Promise<void>(resolve => {
    client.onDidChangeState(() => resolve())
  })

  let watcher: FSWatcher | null = null
  context.subscriptions.push({ dispose: () => watcher?.close() })

  const doReload = async () => {
    // LSPクライアントが起動中なら停止させる。
    if (client.needsStop()) {
      const stateChanged = waitClientStateChange()
      await client.stop()
      await stateChanged // 完全に停止するのを待つ。
    }

    // LSPサーバの実行ファイルをコピーする。(lspBinを直接実行してしまうと変更できなくなるため。)
    await fs.unlink(lspBackupBin).catch(() => undefined)
    await fs.copyFile(lspBin, lspBackupBin).catch(error)
    await fs.access(lspBackupBin)

    // LSPクライアントを起動する。
    const stateChanged = waitClientStateChange()
    client.start()
    await stateChanged

    // LSPサーバの実行ファイルの変更を監視する。
    watcher?.close()
    watcher = watch(lspBin, { persistent: false })
    watcher.once("change", () => requestReload())
    watcher.on("error", error)
  }

  const reload = () => {
    const p = doReload()
    window.setStatusBarMessage("LSPクライアントをリロードしています。", p)
    p.then(() => {
      window.setStatusBarMessage("LSPクライアントがリロードされました。", 5000)
    }, err => {
      error(err)
      log("エラーが発生しました。5秒後にリトライします。")
      setTimeout(() => requestReload(), 5000)
    })
  }

  // リロードを要求する。
  let lastId = 0
  const requestReload = () => {
    const id = ++lastId
    setTimeout(() => {
      if (lastId === id) {
        reload()
      }
    }, THROTTLE)
  }

  requestReload()
  return
}

// -----------------------------------------------
// ライフサイクル
// -----------------------------------------------

/**
 * 拡張機能が起動されたときに自動的に呼ばれる。
 */
export const activate = async (context: ExtensionContext): Promise<void> => {
  if (DEV) {
    dev(context)
    return
  }

  const lspBin = getLspBin(context)
  const client = newLspClient(lspBin)
  context.subscriptions.push(client.start())
}
