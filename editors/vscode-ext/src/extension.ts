import * as ChildProcess from "child_process"
import * as path from "path"
import * as vscode from "vscode"

const pathQuote = (filePath: string) => {
  if (filePath.includes("\"") || !filePath.includes(" ")) {
    return filePath
  }

  return `"${filePath}"`
}

const configGetCompilerPath = (context: vscode.ExtensionContext) => {
  const config = vscode.workspace.getConfiguration("hsp3-ginger")
  return config.get<string>("compilerPath")
}

const configGetHdlPath = (context: vscode.ExtensionContext) => {
  const compilerPath = configGetCompilerPath(context)
  if (!compilerPath) {
    return
  }

  const compilerDir = path.dirname(compilerPath)
  return pathQuote(path.join(compilerDir, "hdl.exe"))
}

const editorGetWord = (textEditor: vscode.TextEditor) => {
  const position = textEditor.selection.start
  const wordRange = textEditor.document.getWordRangeAtPosition(position)
  return textEditor.document.getText(wordRange)
}

const commandHelp = (context: vscode.ExtensionContext) => (textEditor: vscode.TextEditor, edit: vscode.TextEditorEdit) => {
  const hdlPath = configGetHdlPath(context)
  if (!hdlPath) {
    vscode.window.showErrorMessage("Missing hdl.exe: " + hdlPath)
    return
  }

  const word = editorGetWord(textEditor)

  ChildProcess.exec(`${hdlPath} "${word}"`)
}

export const activate = (context: vscode.ExtensionContext) => {
  context.subscriptions.push(vscode.commands.registerTextEditorCommand("hsp3-ginger.help", commandHelp(context)))
}
