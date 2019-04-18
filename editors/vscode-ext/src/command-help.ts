import * as ChildProcess from "child_process"
import * as path from "path"
import * as vscode from "vscode"
import { configGetRoot, configGetHdlPath } from "./config"
import { CouldNotFindCompilerError, CouldNotExecuteError, errorGetMessage } from "./error"

const pathQuote = (filePath: string) => {
  if (filePath.includes("\"") || !filePath.includes(" ")) {
    return filePath
  }

  return `"${filePath}"`
}

const editorGetWord = (textEditor: vscode.TextEditor) => {
  const position = textEditor.selection.start
  const wordRange = textEditor.document.getWordRangeAtPosition(position)
  return textEditor.document.getText(wordRange)
}

const executeGuiApp = async (command: string): Promise<void> => {
  return new Promise((resolve, reject) => {
    ChildProcess.exec(command, err => {
      if (err) {
        return reject(new CouldNotExecuteError(command))
      }
      resolve()
    })
  })
}

export const commandHelp = async (textEditor: vscode.TextEditor) => {
  const config = configGetRoot()
  const hdlPath = configGetHdlPath(config)
  const word = editorGetWord(textEditor)

  // NOTE: HDL が "" を認識しないので単語は "" で囲まない。
  const hdlCommand = `${pathQuote(hdlPath)} ${word}`

  await executeGuiApp(hdlCommand)
}
