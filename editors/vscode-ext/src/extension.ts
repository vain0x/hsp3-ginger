import * as ChildProcess from "child_process"
import * as path from "path"
import * as vscode from "vscode"

class NoCompilerPathError extends Error {}

class CouldNotExecuteError extends Error {}

const handleError = (err: Error) => {
  if (err instanceof NoCompilerPathError) {
    vscode.window.showErrorMessage("コンパイラのファイルパスを設定してください。")
    return
  }

  if (err instanceof CouldNotExecuteError) {
    vscode.window.showErrorMessage(`外部プログラムの起動に失敗しました: ${err.message}`)
    return
  }

  vscode.window.showErrorMessage("何らかのエラーが発生しました。")
}

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
    throw new NoCompilerPathError()
  }

  const compilerDir = path.dirname(compilerPath)
  return pathQuote(path.join(compilerDir, "hdl.exe"))
}

const editorGetWord = (textEditor: vscode.TextEditor) => {
  const position = textEditor.selection.start
  const wordRange = textEditor.document.getWordRangeAtPosition(position)
  return textEditor.document.getText(wordRange)
}

const commandHelp = (context: vscode.ExtensionContext) => async (textEditor: vscode.TextEditor) => {
  try {
    const hdlPath = configGetHdlPath(context)
    const word = editorGetWord(textEditor)

    await new Promise((resolve, reject) => {
      const command = `${hdlPath} ${word}`
      ChildProcess.exec(command, err => {
        if (err) {
          return reject(new CouldNotExecuteError(command))
        }
        resolve()
      })
    })
  } catch (err) {
    handleError(err)
  }
}

export const activate = (context: vscode.ExtensionContext) => {
  context.subscriptions.push(vscode.commands.registerTextEditorCommand("hsp3-ginger.help", commandHelp(context)))
}
