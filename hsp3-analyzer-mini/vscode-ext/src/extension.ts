/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

// Entry point of the VSCode extension.

import * as fs from "fs/promises"
import { watch, FSWatcher } from "fs"
import { ExtensionContext, workspace, window } from "vscode"
import { Disposable, LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient"

/** 開発モード */
const DEV = process.env["HSP3_ANALYZER_MINI_DEV"] === "1"

/** 一定時間待つ。 */
const delay = (timeMs: number) => new Promise<void>(resolve => setTimeout(resolve, timeMs))

/** 非同期処理を行う。失敗したら一定回数だけリトライする。 */
const retrying = async <A>(action: () => Promise<A>, options: { count: number, interval: number }): Promise<A> => {
  let count = options.count
  while (true) {
    if (count <= 0) {
      return await action()
    }

    try {
      return await action()
    } catch {
      // Pass.
    }
    count--
    await delay(options.interval)
  }
}

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

  const DEBOUNCE_TIME = 500
  const RETRY_TIME = 3000
  const RETRY_INTERVAL = 30

  // ログ出力 (開発者ツールのコンソール)
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

  // 通知:
  let notification: Disposable | null = null
  const clearNotification = () => {
    notification?.dispose()
    notification = null
  }

  const notifyStarting = (p: Promise<any>): void => {
    clearNotification()
    window.setStatusBarMessage("LSPクライアントをリロードしています。", p)
  }

  const notifyStarted = (): void => {
    notification = window.setStatusBarMessage("LSPクライアントがリロードされました。", 5000)
  }

  const lspBin = getLspBin(context)
  const lspBackupBin = lspBin.replace(/\.exe$/, "") + "_orig.exe"

  // LSPサーバの実行ファイルをコピーする。(lspBinを直接実行してしまうと変更できなくなるため。)
  const copyLspBin = async () => retrying(async () => {
    await fs.copyFile(lspBin, lspBackupBin)
  }, {
    count: RETRY_TIME / RETRY_INTERVAL,
    interval: RETRY_INTERVAL,
  })

  const client = newLspClient(lspBackupBin)
  context.subscriptions.push({ dispose: () => client.stop() })

  const waitClientStateChange = () => new Promise<void>(resolve => {
    const h = client.onDidChangeState(() => {
      h.dispose()
      resolve()
    })
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

    await copyLspBin()

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
    notifyStarting(p)
    p.then(notifyStarted, error)
  }

  // リロードを要求する。
  let lastId = 0
  const requestReload = () => {
    const id = ++lastId
    setTimeout(() => {
      if (lastId === id) {
        reload()
      }
    }, DEBOUNCE_TIME)
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
