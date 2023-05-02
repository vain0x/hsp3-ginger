import path from "path"
import vscode, { CancellationToken, ConfigurationTarget, DebugConfiguration, DebugConfigurationProvider, Uri, WorkspaceFolder } from "vscode"

/** 開発環境のとき true */
const DEV = process.env["NODE_ENV"] === "development"

// `contributes.debuggers[].type` の値
const HSP3_DEBUG_TYPE = "hsp3"

const configs = vscode.workspace.getConfiguration("hsp3-debug-ginger")

/**
 * 拡張機能がロードされるときに呼ばれる関数
 */
export const activate = (context: vscode.ExtensionContext) => {
  if (DEV) { console.log("[hsp3-debug-ginger] 拡張機能がロードされました") }

  // デバッガーを登録する
  context.subscriptions.push(
    vscode.debug.registerDebugConfigurationProvider(
      HSP3_DEBUG_TYPE,
      new GingerConfigProvider(),
    ))

  // `selectRoot` コマンドを登録する
  context.subscriptions.push(
    vscode.commands.registerCommand("hsp3-debug-ginger.selectRoot", selectHsp3Root),
  )
}

/** デバッグ設定プロバイダー */
class GingerConfigProvider implements DebugConfigurationProvider {
  async resolveDebugConfiguration(folder: WorkspaceFolder | undefined, config: DebugConfiguration, _token?: CancellationToken): Promise<DebugConfiguration | undefined> {
    try {
      // launch.json ファイルがないか、デバッグ構成がないとき
      if (!config.type && !config.request && !config.name) {
        const editor = vscode.window.activeTextEditor
        if (editor && editor.document.languageId === "hsp3") {
          config.type = HSP3_DEBUG_TYPE
          config.name = "Run"
          config.request = "launch"
        }
      }

      config.hsp3Root = await selectHsp3Root()

      if (config.cwd === undefined) {
        if (config.program) {
          config.cwd = path.dirname(config.program)
        } else {
          config.cwd = folder?.uri?.fsPath
        }
        if (!config.cwd) {
          console.warn("[hsp3-debug-ginger] No cwd")
          config.cwd = process.cwd()
        }
      }

      if (config.program === undefined) {
        config.program = path.join(config.cwd, "main.hsp")
      }

      if (config.trace === undefined) {
        config.trace = DEV
      }

      // 後方互換性のため
      config.root = config.hsp3Root

      if (DEV) { console.log(`[hsp3-debug-ginger]: デバッグ設定の解決`, { ...config }) }
      return config
    } catch (err) {
      console.error("[hsp3-debug-ginger] error during config resolution", err)
      throw err
    }
  }

  dispose() { }
}

/** ワークスペースが開かれているディレクトリを取得する */
// const currentWorkspaceDirectory = () => {
//   const workspaceFolders = vscode.workspace.workspaceFolders
//   if (workspaceFolders && workspaceFolders.length > 0) {
//     return workspaceFolders[0].uri.fsPath
//   }
//   throw new Error("no workspace folders")
// }

/**
 * HSP3のインストールディレクトリを選択する
 *
 * - 次の順番で最初に見つかる値を選択する
 *    - `hsp3-debug-ginger.hsp3-root` (VSCodeの設定)
 *    - `hsp3-debug-ginger.root` (VSCodeの設定, 後方互換用)
 *    - `HSP3_ROOT` (環境変数)
 *    - どれでもなければダイアログで選んでもらう
 *        - 選ばれたディレクトリは設定に書き込まれる
 *        - 選ばれなかったときは
 */
const selectHsp3Root = async (): Promise<string | null> => {
  {
    const root = configs.get<string>("hsp3-root")
    if (root) {
      if (DEV) { console.log(`[hsp3-debug-ginger] 設定 hsp3-root='${root}'`) }
      return root
    }
  }

  {
    const root = configs.get<string>("root")
    if (root) {
      if (DEV) { console.log(`[hsp3-debug-ginger] 設定 root='${root}'`) }
      return root
    }
  }

  {
    const root = process.env["HSP3_ROOT"]
    if (root) {
      if (DEV) { console.log(`[hsp3-debug-ginger] 環境変数 HSP3_ROOT='${root}'`) }
      return root
    }
  }

  const paths = await vscode.window.showOpenDialog({
    canSelectFolders: true,
    defaultUri: Uri.parse("file://C:/Program Files"),
    openLabel: "HSPのインストールディレクトリ",
  })

  const selectedPath = paths && paths[0] && paths[0].fsPath
  if (!selectedPath) {
    if (DEV) { console.log("[hsp3-debug-ginger] インストールディレクトリが選択されなかったため、処理はキャンセルされます") }
    return null
  }
  await configs.update("hsp3-root", selectedPath, ConfigurationTarget.Global)

  return selectedPath
}
