import * as fs from "fs"
import * as path from "path"
import { promisify } from "util"
import { window, workspace } from "vscode"
import { selectHsp3Home } from "./ext_command_select_hsp3_home"
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
            || await selectHsp3Home()
        )
    const hsptmpPath = path.join(dirName, "hsptmp")

    // ファイルを作成する。
    const text = activeEditor.document.getText()
    await promisify(fs.writeFile)(hsptmpPath, text)

    return hsptmpPath
}
