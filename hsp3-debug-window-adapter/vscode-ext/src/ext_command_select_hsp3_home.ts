import {
    ConfigurationTarget,
    Uri,
    window,
    workspace,
} from "vscode"
import { HSP3_CONFIG_SECTION, MY_CONFIG_SECTION } from "./ext_constants"
import { DomainError } from "./extension"

const HSP3_HOME_KEY = "hsp3-home"
const HSP3_ROOT_KEY = "hsp3-root" // 廃止予定

const doSelectHsp3Home = async () => {
    // この拡張機能の設定に指定されているものを使う。
    {
        const myConfig = workspace.getConfiguration(MY_CONFIG_SECTION)

        const hsp3Home =
            myConfig.get<string>(HSP3_HOME_KEY)
            ?? myConfig.get<string>(HSP3_ROOT_KEY)

        if (typeof hsp3Home === "string" && hsp3Home !== "") {
            return hsp3Home
        }
    }

    // 言語の設定に定義されているものを使う。
    {
        const myConfig = workspace.getConfiguration(HSP3_CONFIG_SECTION)

        const hsp3Home =
            myConfig.get<string>(HSP3_HOME_KEY)
            ?? myConfig.get<string>(HSP3_ROOT_KEY)

        if (typeof hsp3Home === "string" && hsp3Home !== "") {
            return hsp3Home
        }
    }

    // 環境変数に定義されているものを使う。
    {
        const hsp3Home = process.env["HSP3_HOME"] ?? process.env["HSP3_ROOT"]
        if (typeof hsp3Home === "string" && hsp3Home !== "") {
            return hsp3Home
        }
    }

    // 選択してもらう。
    const paths = await window.showOpenDialog({
        canSelectFolders: true,
        defaultUri: Uri.parse("file:///C:/Program Files (x86)"),
        openLabel: "HSP3 のインストールディレクトリ",
    })

    const selectedPath = paths && paths[0] && paths[0].fsPath
    if (selectedPath) {
        // 選択結果をユーザー設定に保存する。
        {
            const myConfig = workspace.getConfiguration(MY_CONFIG_SECTION)
            await myConfig.update(HSP3_HOME_KEY, selectedPath, ConfigurationTarget.Global)
        }
        return selectedPath
    }

    return null
}

export const selectHsp3Home = async () => {
    const hsp3Home = await doSelectHsp3Home()
    if (!hsp3Home) {
        throw new DomainError("HSP3 のインストールディレクトリが指定されていません。")
    }

    return hsp3Home
}
