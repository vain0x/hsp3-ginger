// 簡易的な Debug Adapter Protocol (DAP) アダプタの実装。

import { ChildProcess, execFile, spawn } from "child_process"
import * as fs from "fs/promises"
import { appendFileSync } from "fs"
import * as path from "path"
import { InitializedEvent, LoggingDebugSession, TerminatedEvent } from "@vscode/debugadapter";
import { DebugProtocol } from "@vscode/debugprotocol"

/**
 * デバッグの開始時に開発ツール (VSCode) から渡されるデータ。
 */
interface LaunchRequestArguments extends DebugProtocol.LaunchRequestArguments {
    /**
     * 実行する HSP3 スクリプトのファイルパス
     */
    program: string

    /**
     * HSP3 のインストールディレクトリ (絶対パス)
     */
    hsp3Root: string

    /** 設定項目 'utf8Support' の値 */
    utf8Support: string

    /**
     * デバッグアダプターのあるディレクトリ (絶対パス)
     *
     * このファイルからみて ../dist のこと。
     */
    distDir: string

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
 * ビルダーをコンパイルする
 *
 * - dist/builder.md を参照
 * - 生成されたファイルは拡張機能のディレクトリに保存される
 *
 * @returns 生成されたオブジェクトファイルのパス
 */
const compileBuilder = async (hsp3Root: string, distDir: string): Promise<string> => {
    const hspcmpExe = path.join(hsp3Root, "hspcmp.exe")
    const builderAx = path.join(distDir, "builder.ax")
    const builderHsp = path.join(distDir, "builder.hsp")

    // コンパイル済みならスキップ。
    if (!traceLogEnabled && await fileExists(builderAx)) {
        return builderAx
    }

    writeTrace("compile builder", { hsp3Root, distDir, builderHsp })

    const result = await new Promise((resolve, reject) => {
        execFile(
            hspcmpExe,
            [
                `--compath=${hsp3Root}\\common\\`,
                builderHsp,
            ],
            {
                cwd: distDir,
                timeout: TIMEOUT_MILLIS,
            },
            (err, stdout, stderr) => {
                if (err) { reject(err); return }
                resolve({ stdout, stderr })
            })
    })

    writeTrace("compile builder result", result)

    if (!await fileExists(builderAx)) {
        traceLogEnabled = true
        writeTrace("step1 no object file")
        throw new Error("ビルダーのコンパイルに失敗しました (ログを確認してください " + traceLogFile + ")")
    }
    return builderAx
}

/**
 * スクリプトをコンパイルして、オブジェクトファイルを生成する。
 */
const compileHsp = async (program: string, hsp3Root: string, utf8Support: string, distDir: string) => {
    const hsp3clExe = path.join(hsp3Root, "hsp3cl.exe")
    const builderAx = await compileBuilder(hsp3Root, distDir)

    const builderArgs = [
        builderAx,
        "--hsp",
        hsp3Root,
        "compile",
        program,
    ]

    if (utf8Support === "enabled" || utf8Support === "input") {
        builderArgs.push("--utf8-input")
    }
    if (utf8Support === "enabled" || utf8Support === "output") {
        builderArgs.push("--utf8-output")
    }

    const workDir = path.dirname(program)
    const objName = path.join(workDir, "start.ax")

    writeTrace("spawn builder", { hsp3clExe, builderArgs, workDir })

    // ビルダーを起動する。これによりオブジェクトファイルが生成される。
    const { success, stdout, stderr } = await new Promise<{ success: boolean, stdout: string, stderr: string }>(resolve => {
        execFile(
            hsp3clExe,
            builderArgs,
            {
                cwd: workDir,
                timeout: TIMEOUT_MILLIS,
            },
            (err, stdout, stderr) => {
                resolve({ success: !err, stdout, stderr })
            })
    })

    writeTrace("builder result", { success, stdout, stderr })

    let output: string
    if (stderr == "") {
        output = stdout
    } else {
        output = "[STDOUT]\r\n" + stdout + "\r\n[STDERR]\r\n" + stderr
    }

    // ビルダーの出力から、使うランタイムを特定する。
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
        success,
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
            traceLogFile = path.join(args.distDir, TRACE_LOG_FILE_NAME)
        }

        writeTrace("launch", args)

        // 正しく引数が渡されたか検査する。
        const { program, hsp3Root, utf8Support, distDir } = args

        if (typeof program !== "string"
            || typeof hsp3Root !== "string"
            || typeof utf8Support !== "string"
            || typeof distDir !== "string") {
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
        const compileResult = await compileHsp(program, hsp3Root, utf8Support, distDir)
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
        if (!success) {
            response.message = message
        }
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
