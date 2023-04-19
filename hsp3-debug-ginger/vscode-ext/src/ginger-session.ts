import * as path from "path"
import {
  Logger,
  logger,
  LoggingDebugSession,
  InitializedEvent,
  TerminatedEvent,
  StoppedEvent,
  BreakpointEvent,
  OutputEvent,
  Thread,
  StackFrame,
  Source,
  Breakpoint,
  ContinuedEvent,
  Scope,
  Variable,
} from '@vscode/debugadapter';
import { DebugProtocol } from '@vscode/debugprotocol';
import { basename } from 'path';
import { GingerConnectionServer } from './ginger-connection';
const { Subject } = require('await-notify');
import { spawn } from "child_process"

interface HspDebugResponseBreak {
  type: "stop",
  file: string | undefined,
  line: number,
}

/**
 * HSP デバッガーから送られてくるメッセージ
 */
type HspDebugResponse =
  | HspDebugResponseBreak
  | { type: "continue" }
  | { type: "globals", vars: Array<{ name: string, value: string }> }

interface LaunchRequestArguments extends DebugProtocol.LaunchRequestArguments {
  /** カレントディレクトリ (現在はワークスペースのルートディレクトリが入る) */
  cwd: string
  /** HSP のインストールディレクトリ */
  root: string
  /** 最初に実行するスクリプトのファイルパス。(例: main.hsp) */
  program: string
  /** 詳細なログを出力するなら true (デバッグ用) */
  trace: boolean
}

const THREAD_ID = 1
const THREADS = [new Thread(THREAD_ID, "Main thread")]

const GLOBAL_SCOPE_REF = 1
const GLOBAL_SCOPE: Scope = {
  name: "グローバル",
  variablesReference: GLOBAL_SCOPE_REF,
  expensive: true,
}

export class GingerDebugSession extends LoggingDebugSession {
  private configDone = new Subject()
  private server: GingerConnectionServer | undefined
  private currentFile: string = "main.hsp"
  private currentLine: number = 1
  private options: LaunchRequestArguments | undefined

  public constructor() {
    super(path.resolve("ginger-session.txt"))

    logger.warn("LOG: " + path.resolve("ginger-session.txt"))

    this.setDebuggerLinesStartAt1(true)
    this.setDebuggerColumnsStartAt1(true)
  }

  protected initializeRequest(
    response: DebugProtocol.InitializeResponse,
    _args: DebugProtocol.InitializeRequestArguments
  ): void {
    response.body = response.body || {};
    response.body.supportsConfigurationDoneRequest = true

    this.sendResponse(response)
    this.sendEvent(new InitializedEvent())
  }

  protected configurationDoneRequest(
    response: DebugProtocol.ConfigurationDoneResponse,
    args: DebugProtocol.ConfigurationDoneArguments
  ): void {
    super.configurationDoneRequest(response, args)
    this.configDone.notify()
  }

  protected async launchRequest(
    response: DebugProtocol.LaunchResponse,
    args: LaunchRequestArguments
  ) {
    this.options = args

    logger.setup(args.trace ? Logger.LogLevel.Verbose : Logger.LogLevel.Stop, false)
    await this.configDone.wait(1000)

    logger.verbose(JSON.stringify(args))

    this.startServer(args.cwd)
    this.sendResponse(response)

    // FIXME: サーバーが起動したときに resolve する。
    await new Promise<void>(resolve => setTimeout(resolve, 500));
    this.startProgram(args)
  }

  protected disconnectRequest(
    response: DebugProtocol.DisconnectResponse,
    args: DebugProtocol.DisconnectArguments
  ): void {
    super.disconnectRequest(response, args)

    if (this.server) {
      this.server.stopServer()
      this.server = undefined
    }

    this.sendResponse(response);
  }

  protected threadsRequest(response: DebugProtocol.ThreadsResponse): void {
    response.body = { threads: THREADS }
    this.sendResponse(response)
  }

  protected stackTraceRequest(
    response: DebugProtocol.StackTraceResponse,
    args: DebugProtocol.StackTraceArguments,
  ): void {
    const stackFrames = this.currentStack(args.startFrame || 0)
    response.body = {
      stackFrames,
      totalFrames: stackFrames.length,
    }
    this.sendResponse(response)
  }

  protected continueRequest(response: DebugProtocol.ContinueResponse, _args: DebugProtocol.ContinueArguments): void {
    logger.log("操作 再開")
    this.request({ type: "continue" })
    this.sendResponse(response)
  }

  protected nextRequest(response: DebugProtocol.NextResponse, _args: DebugProtocol.NextArguments): void {
    logger.log("操作 次へ")
    this.request({ type: "next" })
    this.sendResponse(response)
  }

  protected pauseRequest(response: DebugProtocol.PauseResponse, _args: DebugProtocol.PauseArguments): void {
    logger.log("操作 中断")
    this.request({ type: "pause" })
    this.sendStop("pause")
    this.sendResponse(response)
  }

  protected scopesRequest(
    response: DebugProtocol.ScopesResponse,
    _args: DebugProtocol.ScopesArguments
  ): void {
    response.success = true
    response.body = {
      scopes: [GLOBAL_SCOPE]
    }
    this.sendResponse(response)
  }

  protected variablesRequest(
    response: DebugProtocol.VariablesResponse,
    _args: DebugProtocol.VariablesArguments
  ): void {
    this.variablesResponses.push(response)
    this.request({ type: "globals" })
  }

  private variablesResponses: DebugProtocol.VariablesResponse[] = []

  // protected evaluateRequest(response: DebugProtocol.EvaluateResponse, args: DebugProtocol.EvaluateArguments): void {
  // }

  /**
   * デバッガーにリクエストを送信する。
   */
  private request(event: { type: "pause" | "continue" | "next" | "globals" }): void {
    const server = this.server;
    if (server === undefined) {
      logger.warn(`操作 失敗 サーバーが起動していません ${JSON.stringify(event)}`)
      return;
    }

    server.send(JSON.stringify(event));
  }

  /**
   * デバッガーからのリクエストに応答する。
   */
  private handleRequest(message: string) {
    const event = JSON.parse(message) as HspDebugResponse;
    switch (event.type) {
      case "stop":
        this.sendStop("breakpoint")
        this.currentFile = event.file || this.currentFile
        this.currentLine = event.line
        break;
      case "continue":
        this.sendContinue()
        break;
      case "globals":
        {
          const response = this.variablesResponses.pop()
          if (!response) {
            logger.warn("variablesResponse への応答を受信しましたが、送信すべきレスポンスがありません。")
            return
          }

          response.body = {
            variables: event.vars.map(v => ({ ...v, variablesReference: 0 } as Variable))
          }
          this.sendResponse(response)
          break
        }
      default: {
        logger.warn(`デバッグクライアント 不明なメッセージ ${message}`)
        return
      }
    }
  }

  dummy() {
    this.sendOutput("", 0, 0, "")
    this.sendBreakpointValidated(0, {} as any)
    this.sendTerminated()
  }

  private sendOutput(
    filePath: string,
    line: number,
    column: number,
    text: string,
  ) {
    const e: DebugProtocol.OutputEvent = new OutputEvent(`${text}\n`);
    e.body.source = this.createSource(filePath);
    e.body.line = this.convertDebuggerLineToClient(line);
    e.body.column = this.convertDebuggerColumnToClient(column);
    this.sendEvent(e);
  }

  private sendBreakpointValidated(id: number, bp: Breakpoint) {
    this.sendEvent(new BreakpointEvent('changed', <DebugProtocol.Breakpoint>{ verified: bp.verified, id }));
  }

  private sendStop(reason: "entry" | "pause" | "step" | "breakpoint" | "stopOnException") {
    this.sendEvent(new StoppedEvent(reason, THREAD_ID))
  }

  private sendContinue() {
    this.sendEvent(new ContinuedEvent(THREAD_ID))
  }

  private sendTerminated() {
    this.sendEvent(new TerminatedEvent())
  }

  private createSource(filePath: string): Source {
    // FIXME: common からの相対パスも許容する
    const fullPath = path.resolve(this.options!.cwd, filePath)
    const clientPath = this.convertDebuggerPathToClient(fullPath)
    return new Source(basename(filePath), clientPath, undefined, undefined, {})
  }

  private startServer(cwd: string) {
    logger.verbose(JSON.stringify({ cwd }));

    const server = new GingerConnectionServer(m => this.handleRequest(m));
    this.server = server;
    server.start();
  }

  /// HSP のデバッグ実行を開始する。
  private startProgram(args: LaunchRequestArguments) {
    const entryPath = path.resolve(args.cwd, args.program)
    const runtimePath = path.resolve(args.cwd, args.root, "chspcomp.exe")
    const p = spawn(runtimePath, ["/diw", entryPath], { cwd: args.cwd })
    p.on("close", () => {
      console.log("chspcomp:  close")
    })
    p.on("exit", code => {
      console.log("chspcomp: exited", code)
    })
    p.on("error", () => {
      console.error("chspcomp: error", { runtimePath, entryPath, cwd: args.cwd })
    })
  }

  private currentStack(startFrame: number) {
    const frame = new StackFrame(
      startFrame,
      "main",
      this.createSource(this.currentFile),
      this.convertDebuggerLineToClient(this.currentLine),
    )
    return [frame]
  }
}
