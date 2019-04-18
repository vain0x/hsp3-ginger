import * as vscode from "vscode"
import { commandHelp } from "./command-help"
import { errorGetMessage } from "./error"

const handleError = (err: Error) => {
  console.error(err)
  vscode.window.showErrorMessage(errorGetMessage(err))
}

const commandCatch = <T extends any[], U>(command: (...args: T) => U) => (...args: T) => {
  Promise.resolve(command(...args)).catch(handleError)
}

export const activate = (context: vscode.ExtensionContext) => {
  context.subscriptions.push(vscode.commands.registerTextEditorCommand("hsp3-ginger.help", commandCatch(commandHelp)))
}
