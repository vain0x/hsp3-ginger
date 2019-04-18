import * as path from "path"
import * as vscode from "vscode"
import { CouldNotFindCompilerError } from "./error"

export const configGetRoot = (): vscode.WorkspaceConfiguration =>
  vscode.workspace.getConfiguration("hsp3-ginger")

const configGetCompilerPath = (config: vscode.WorkspaceConfiguration) =>
  config.get<string>("compilerPath")

export const configGetHdlPath = (config: vscode.WorkspaceConfiguration) => {
  const compilerPath = configGetCompilerPath(config)
  if (!compilerPath) {
    throw new CouldNotFindCompilerError()
  }

  const compilerDir = path.dirname(compilerPath)
  return path.join(compilerDir, "hdl.exe")
}
