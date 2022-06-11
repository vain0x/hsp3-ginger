import * as fsP from "fs/promises"
import * as path from "path"
import { window, workspace } from "vscode"
import { selectHsp3Root } from "./ext_command_select_hsp3_root"
import { DomainError } from "./extension"

/**
 * 作業ディレクトリに hsptmp ファイルを生成する。
 */
export const createHsptmp = async () => {
    const activeEditor = window.activeTextEditor
    if (!activeEditor) {
        throw new DomainError("エディターが開かれていません。")
    }

    // ファイルパスを構成する。
    let fileName = activeEditor.document.fileName
    const dirName = fileName
        ? path.dirname(fileName)
        : (
            workspace.workspaceFolders
            && workspace.workspaceFolders.length >= 1
            && workspace.workspaceFolders[0].uri.fsPath
            || await selectHsp3Root()
        )
    const hsptmpPath = path.join(dirName, "hsptmp")

    // ファイルを作成する。
    const text = activeEditor.document.getText()
    await fsP.writeFile(hsptmpPath, text)

    return hsptmpPath
}
