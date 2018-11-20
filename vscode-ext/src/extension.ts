import * as vscode from "vscode";
import {
  WorkspaceFolder,
  DebugConfiguration,
  ProviderResult,
  CancellationToken,
} from "vscode";
import { Hsp3DebugType } from "./constants";

export function activate(context: vscode.ExtensionContext) {
  const provider = new GingerConfigProvider()
  context.subscriptions.push(vscode.debug.registerDebugConfigurationProvider(Hsp3DebugType, provider))
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
  ): ProviderResult<DebugConfiguration> {
    return (async () => {

      const { workspaceFolders } = vscode.workspace;
      if (workspaceFolders && workspaceFolders.length > 0) {
        config.cwd = workspaceFolders[0].uri.fsPath
      }

      config.hspPath = await this.askHSPPath()

      return config
    })()
  }

  private async askHSPPath() {
    const config = vscode.workspace.getConfiguration("hsp3DebugGinger")
    const PATH_KEY = "path"

    const hspPath = config.get(PATH_KEY)
    if (typeof hspPath === "string" && hspPath !== "") {
      return hspPath
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

    await config.update(PATH_KEY, selectedPath, vscode.ConfigurationTarget.Global)
    return selectedPath
  }

  dispose() {
    // pass
  }
}
