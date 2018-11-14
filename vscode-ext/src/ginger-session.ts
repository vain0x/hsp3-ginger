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
} from 'vscode-debugadapter';
import { DebugProtocol } from 'vscode-debugprotocol';
import { basename } from 'path';
import { GingerConnectionServer } from './ginger-connection';
const { Subject } = require('await-notify');

interface HspDebugResponseBreak {
  type: "stopOnBreakpoint",
  line: number,
  column: number,
  message: string,
}

/**
 * HSP デバッガーから送られてくるメッセージ
 */
type HspDebugResponse =
  | HspDebugResponseBreak
  | { type: "continue" }

interface LaunchRequestArguments extends DebugProtocol.LaunchRequestArguments {
  cwd: string
  trace: boolean
}

const THREAD_ID = 1
const THREADS = [new Thread(THREAD_ID, "Main thread")]

export class GingerDebugSession extends LoggingDebugSession {
  private configDone = new Subject()
  private server: GingerConnectionServer | undefined
  private currentLint: number = 2
  private cwd: string = path.resolve(".")

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
    this.cwd = args.cwd;
    logger.setup(args.trace ? Logger.LogLevel.Verbose : Logger.LogLevel.Stop, false)

    await this.configDone.wait(1000)

    this.startServer(args.cwd)
    this.sendResponse(response)
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
    this.request({ event: "continue" })
    this.sendResponse(response)
  }

  protected nextRequest(response: DebugProtocol.NextResponse, _args: DebugProtocol.NextArguments): void {
    logger.log("操作 次へ")
    this.request({ event: "step" })
    this.sendResponse(response)
  }

  protected pauseRequest(response: DebugProtocol.PauseResponse, _args: DebugProtocol.PauseArguments): void {
    logger.log("操作 中断")
    this.request({ event: "pause" })
    this.sendStop("pause")
    this.sendResponse(response)
  }

  // protected scopesRequest(
  //   response: DebugProtocol.ScopesResponse,
  //   args: DebugProtocol.ScopesArguments
  // ): void {
  // }

  // protected variablesRequest(
  //   response: DebugProtocol.VariablesResponse,
  //   args: DebugProtocol.VariablesArguments
  // ): void {
  //   this.sendResponse(response);
  // }

  // protected evaluateRequest(response: DebugProtocol.EvaluateResponse, args: DebugProtocol.EvaluateArguments): void {
  // }

  /**
   * デバッガーにリクエストを送信する。
   */
  private request(event: { event: "pause" | "continue" | "step" }): void {
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
      case "stopOnBreakpoint":
        this.sendStop("breakpoint")
        this.currentLint = event.line
        break;
      case "continue":
        this.sendContinue()
        break;
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
    const fullPath = path.resolve(this.cwd, filePath)
    const clientPath = this.convertDebuggerPathToClient(fullPath)
    return new Source(basename(filePath), clientPath, undefined, undefined, {})
  }

  private startServer(cwd: string) {
    logger.verbose(JSON.stringify({ cwd }));

    const server = new GingerConnectionServer(m => this.handleRequest(m));
    this.server = server;
    server.start();
  }

  private currentStack(startFrame: number) {
    const frame = new StackFrame(
      startFrame,
      "main",
      this.createSource("main.hsp"),
      this.convertDebuggerLineToClient(this.currentLint),
    )
    return [frame]
  }
}
