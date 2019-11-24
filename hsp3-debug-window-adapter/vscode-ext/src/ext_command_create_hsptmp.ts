import * as fs from "fs"
import * as path from "path"
import { promisify } from "util"
import { window, workspace } from "vscode"
import { selectHsp3Root } from "./ext_command_select_hsp3_root"

/**
 * 作業ディレクトリに hsptmp ファイルを生成する。
 */
export const createHsptmpCommand = async () => {
    const activeEditor = window.activeTextEditor
    if (!activeEditor) {
        window.showWarningMessage("エディターが開かれていません。")
        return
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
    if (!dirName) {
        return null
    }
    const hsptmpPath = path.join(dirName, "hsptmp")

    // ファイルを作成する。
    const text = activeEditor.document.getText()
    await promisify(fs.writeFile)(hsptmpPath, text)

    return hsptmpPath
}
