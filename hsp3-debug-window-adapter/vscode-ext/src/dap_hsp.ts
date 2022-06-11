// 簡易的な Debug Adapter Protocol (DAP) アダプタの実装。

import { ChildProcess, exec, spawn } from "child_process"
import * as fs from "fs/promises"
import { appendFileSync } from "fs"
import * as path from "path"
import { promisify } from "util"
import {
    InitializedEvent,
    LoggingDebugSession,
    TerminatedEvent,
} from "@vscode/debugadapter";
import { DebugProtocol } from "@vscode/debugprotocol"

/**
 * デバッグの開始時に開発ツール (VSCode) から渡されるデータ。
 */
interface LaunchRequestArguments extends DebugProtocol.LaunchRequestArguments {
    /**
     * 実行する HSP3 スクリプトのファイルパス (基本的には hsptmp への絶対パス)
     */
    program: string

    /**
     * HSP3 のインストールディレクトリ (絶対パス)
     */
    hsp3Root: string

    /** utf-support */
    utf8Support: string

    /**
     * デバッグアダプターのあるディレクトリ (絶対パス)
     *
     * このファイルからみて ../out のこと。
     */
    outDir: string

    trace?: boolean
}

/**
 * [開発用] ファイルにログ出力する。
 */
let traceLogEnabled = process.env.HSP3_DEBUG_WINDOW_ADAPTER_DEBUG === "1"

const TRACE_LOG_FILE_NAME = "hsp3-debug-window-adapter.log"

let traceLogFile = traceLogEnabled
    ? path.join(__dirname, TRACE_LOG_FILE_NAME)
    : TRACE_LOG_FILE_NAME

/**
 * コンパイルの各ステージのタイムアウト時間
 */
const TIMEOUT_MILLIS = 15 * 1000

/**
 * デバッグ用にログを出力する。
 */
const writeTrace = (msg: string, data?: unknown) => {
    if (!traceLogEnabled) {
        return
    }

    if (data !== undefined) {
        if (data != null && data instanceof Error) {
            data = data.toString()
        }
        if (typeof data !== "string" || data.includes("\0")) {
            data = JSON.stringify(data, undefined, 2)
        }
        msg += "\ndata: " + data
    }
    msg += "\n\n"

    appendFileSync(traceLogFile, msg)
}

/**
 * ファイルが存在するか？
 */
const fileExists = async (fileName: string): Promise<boolean> =>
    // ファイルへのアクセス権があれば真、エラーだったら偽
    await fs.access(fileName).then(() => true, () => false)

/**
 * hsp3_build (ビルダー) の実行ファイルを作成する。
 *
 * 実行ファイルは拡張機能のディレクトリに保存されるので、処理は1回だけ行う。
 *
 * @returns 生成された実行ファイルのパス
 */
const buildBuilder = async (hsp3Root: string, outDir: string): Promise<string> => {
    const hspcmpExe = path.join(hsp3Root, "hspcmp.exe")
    const hspcmpDllSrc = path.join(hsp3Root, "hspcmp.dll")
    const hspcmpDllDest = path.join(outDir, "hspcmp.dll")
    const hsp3clExe = path.join(hsp3Root, "hsp3cl.exe")
    const hsp3BuildHsp = path.join(outDir, "hsp3_build_cli.hsp")
    const hsp3BuildAx = path.join(outDir, "hsp3_build_cli.ax")
    const hsp3BuildExe = path.join(outDir, "hsp3_build_cli.exe")
    const compathArg = `--compath=${hsp3Root}/common/`

    // ビルド済みならスキップ。
    if (!traceLogEnabled && await fileExists(hsp3BuildExe)) {
        return hsp3BuildExe
    }

    writeTrace("build hsp3_build", { hsp3Root, outDir })

    // ビルダーが使う hspcmp.dll をコピーする。
    writeTrace("copy compiler", { hspcmpDllSrc, hspcmpDllDest })
    await fs.copyFile(hspcmpDllSrc, hspcmpDllDest)

    // ステージ1: hspcmp.exe でコンパイルし、オブジェクトファイルを作る。
    const cmd1 = `"${hspcmpExe}" "${compathArg}" "${hsp3BuildHsp}"`
    writeTrace("stage1 cmd", cmd1)

    const result1 = await promisify(exec)(cmd1, {
        cwd: outDir,
        timeout: TIMEOUT_MILLIS,
    })

    writeTrace("stage1 result", result1)

    writeTrace("wait for the object file to be written")
    for (let i = 0; i < 30; i++) {
        if (await fileExists(hsp3BuildAx)) {
            writeTrace("object written")
            break
        }
        await new Promise(resolve => setTimeout(resolve, 100))
    }

    if (!await fileExists(hsp3BuildAx)) {
        writeTrace("object file missing")
    }

    // ステージ2: ランタイムにオブジェクトファイルを渡して実行し、実行ファイルを作る。
    const cmd2 = `"${hsp3clExe}" "${hsp3BuildAx}" make --hsp "${hsp3Root}" "${hsp3BuildHsp}"`
    writeTrace("stage2 cmd", cmd2)

    const result2 = await promisify(exec)(cmd2, {
        cwd: outDir,
        timeout: TIMEOUT_MILLIS,
    })

    writeTrace("stage2 result", result2)

    return hsp3BuildExe
}

/**
 * スクリプトをコンパイルして、オブジェクトファイルを生成する。
 */
const compileHsp = async (program: string, hsp3Root: string, utf8Support: string, outDir: string) => {
    const builderExe = await buildBuilder(hsp3Root, outDir)

    const builderArgs = [
        "--hsp",
        hsp3Root,
        "compile",
        program,
    ]

    if (utf8Support !== "enabled") {
        if (utf8Support === "disabled" || utf8Support === "output") {
            builderArgs.push("--no-utf8-input")
        }
        if (utf8Support === "disabled" || utf8Support === "input") {
            builderArgs.push("--no-utf8-output")
        }
    }

    const workDir = path.dirname(program)
    const objName = path.join(workDir, "start.ax")

    writeTrace("spawn builder", { builderExe, builderArgs, workDir })

    // ビルドツールを起動・監視する。これによりオブジェクトファイルが生成されるはず。
    const [stdout, stderr, exitCode] = await new Promise<[string, string, number | null]>((resolve, reject) => {
        const builderProcess = spawn(
            builderExe,
            builderArgs,
            {
                cwd: workDir,
                stdio: "pipe",
                timeout: TIMEOUT_MILLIS,
            })

        const stdout: Buffer[] = []
        const stderr: Buffer[] = []

        builderProcess.stdout.on("data", data => {
            stdout.push(data)
        })

        builderProcess.stderr.on("data", data => {
            stderr.push(data)
        })

        builderProcess.on("close", code => {
            resolve([
                stdout.map(b => b.toString()).join(""),
                stderr.map(b => b.toString()).join(""),
                code
            ])
        })

        builderProcess.on("error", err => {
            writeTrace("builder emit error", err)
            reject(err)
        })
    })

    let output = stdout
    if (stderr !== "") {
        output += "\r\nERROR: " + stderr
    }

    // ビルドツールの出力から、使うランタイムを特定する。
    const RUNTIME_REGEXP = /#Use runtime "([a-zA-Z_0-9.]+)"/
    let runtimeName = "hsp3.exe"
    {
        const m = output.match(RUNTIME_REGEXP)
        writeTrace("runtime search", m)
        if (m && m.length >= 1) {
            runtimeName = m[1]
        }
    }

    return {
        success: exitCode === 0,
        runtimePath: path.join(hsp3Root, runtimeName),
        objName,
        output,
    }
}

export class Hsp3DebugSession extends LoggingDebugSession {
    /**
     * デバッグ実行のために起動されたランタイムのプロセス
     */
    private _debuggeeProcess: ChildProcess | null = null

    constructor() {
        super(traceLogFile)

        writeTrace("new session", {
            cwd: process.cwd(),
            args: process.argv,
        })
    }

    /**
     * デバッガーの初期化
     *
     * まだデバッグ実行は開始しない。
     */
    public initializeRequest(response: DebugProtocol.InitializeResponse, args: DebugProtocol.InitializeRequestArguments): void {
        writeTrace("initialize", args)

        response.body = response.body || {}

        this.sendResponse(response)

        this.sendEvent(new InitializedEvent())
    }

    private async _doLaunch(args: LaunchRequestArguments): Promise<[boolean, string]> {
        if (args && args.trace) {
            traceLogEnabled = true
            traceLogFile = path.join(args.outDir, TRACE_LOG_FILE_NAME)
        }

        writeTrace("launch", args)

        // 正しく引数が渡されたか検査する。
        const { program, hsp3Root, utf8Support, outDir } = args

        if (typeof program !== "string"
            || typeof hsp3Root !== "string"
            || typeof utf8Support !== "string"
            || typeof outDir !== "string") {
            writeTrace("bad arguments")
            return [false, "デバッガーの起動に失敗しました。(launch 引数が不正です。)"]
        }

        // HSP3 のインストールディレクトリが正しいパスか検査する。
        const hspcmpExe = path.join(hsp3Root, "hspcmp.exe")
        writeTrace("verify hsp3Root", { hspcmpExe })
        if (!await fileExists(hspcmpExe)) {
            return [false, `コンパイルを開始できません。指定されたHSPのディレクトリ: ${hsp3Root}`]
        }

        // コンパイルする。
        writeTrace("compile")
        const compileResult = await compileHsp(program, hsp3Root, utf8Support, outDir)
        writeTrace("compiled", { compileResult })

        if (!compileResult.success) {
            return [false, `コンパイルエラーが発生しました。\r\n${compileResult.output}`]
        }

        // ランタイムを起動・監視する。
        const { runtimePath, objName } = compileResult
        const runtimeArgs = [objName]

        writeTrace("spawn debuggee")

        this._debuggeeProcess = spawn(
            runtimePath,
            runtimeArgs,
            {
                cwd: path.dirname(program),
                stdio: "pipe",
                windowsHide: false,
            })

        this._debuggeeProcess.on("close", exitCode => {
            writeTrace("debuggee on close", { exitCode })
            this._doShutdown({ exitCode })
        })

        this._debuggeeProcess.on("error", err => {
            writeTrace("debuggee on error", { err })
            this._doShutdown({ err })
        })

        writeTrace("spawned")
        return [true, ""]
    }

    /**
     * デバッグの開始が要求されたとき
     */
    public async launchRequest(response: DebugProtocol.LaunchResponse, args: LaunchRequestArguments) {
        const [success, message] = await this._doLaunch(args).catch(err => [false, err.toString()])

        response.success = success
        response.message = message
        this.sendResponse(response)
    }

    /**
     * デバッグの停止が要求されたとき
     */
    public terminateRequest(response: DebugProtocol.TerminateResponse, args: DebugProtocol.TerminateArguments) {
        writeTrace("terminate", args)
        const process = this._debuggeeProcess

        if (process) {
            writeTrace("kill")
            process.kill()
            this._debuggeeProcess = null
        }

        response.success = true
        this.sendResponse(response)
    }

    /**
     * デバッグを停止する。
     */
    private _doShutdown(data: unknown) {
        writeTrace("shutdown", data)

        this.sendEvent(new TerminatedEvent())
        this._debuggeeProcess = null
    }
}
