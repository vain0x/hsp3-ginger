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
    {
      const { workspaceFolders } = vscode.workspace;
      if (workspaceFolders && workspaceFolders.length > 0) {
        config.cwd = workspaceFolders[0].uri.fsPath
      }
    }

    return config
  }

  dispose() {
    // pass
  }
}
