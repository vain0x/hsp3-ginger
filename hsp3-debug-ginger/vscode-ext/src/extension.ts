import * as vscode from "vscode";
import {
  WorkspaceFolder,
  DebugConfiguration,
  CancellationToken,
} from "vscode";
import { Hsp3DebugType } from "./constants";
import * as path from "path";

const configs = vscode.workspace.getConfiguration("hsp3-debug-ginger")

export function activate(context: vscode.ExtensionContext) {
  const provider = new GingerConfigProvider()
  context.subscriptions.push(vscode.debug.registerDebugConfigurationProvider(Hsp3DebugType, provider))
  context.subscriptions.push(vscode.commands.registerCommand("hsp3-debug-ginger.selectRoot", selectRoot))
  context.subscriptions.push(provider)
}

export function deactivate() {
  // pass
}

class GingerConfigProvider implements vscode.DebugConfigurationProvider {
  resolveDebugConfiguration(
    _folder: WorkspaceFolder | undefined,
    config: DebugConfiguration,
    _token?: CancellationToken
  ): Promise<DebugConfiguration> {
    return (async () => {
      config.cwd = calcCwd()
      config.root = await selectRoot()

      if (config.trace === undefined) {
        config.trace = false
      }
      if (config.program === undefined) {
        config.program = path.join(config.cwd, "main.hsp")
      }
      return config
    })()
  }

  dispose() {
    // pass
  }
}

const calcCwd = () => {
  const { workspaceFolders } = vscode.workspace;
  if (workspaceFolders && workspaceFolders.length > 0) {
    return workspaceFolders[0].uri.fsPath
  }
  throw new Error("could not calculate cwd")
}

const selectRoot = async () => {
  const ROOT_KEY = "root"

  const root = configs.get(ROOT_KEY)
  if (typeof root === "string" && root !== "") {
    return root
  }

  const paths = await vscode.window.showOpenDialog({
    canSelectFolders: true,
    defaultUri: vscode.Uri.parse("file://C:/Program Files"),
    openLabel: "HSPのインストールディレクトリ",
  })

  const selectedPath = paths && paths[0] && paths[0].fsPath
  if (!selectedPath) {
    vscode.window.showErrorMessage("HSPのインストールディレクトリが指定されていません。")
    throw new Error("Configuration failed.")
  }

  await configs.update(ROOT_KEY, selectedPath, vscode.ConfigurationTarget.Global)
  return selectedPath
}
