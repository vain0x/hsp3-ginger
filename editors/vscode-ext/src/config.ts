import * as path from "path"
import * as vscode from "vscode"

export const configGetRoot = (): vscode.WorkspaceConfiguration =>
  vscode.workspace.getConfiguration("hsp3-ginger")

export const configGetCompilerPath = (config: vscode.WorkspaceConfiguration) =>
  config.get<string>("compilerPath")!

const configGetHspDir = (config: vscode.WorkspaceConfiguration) => {
  return path.dirname(configGetCompilerPath(config))
}

export const configGetHdlPath = (config: vscode.WorkspaceConfiguration) => {
  return path.join(configGetHspDir(config), "hdl.exe")
}

export const configGetDebugCompilerOptions = (config: vscode.WorkspaceConfiguration) =>
  config.get<string[]>("debugCompilerOptions")!
