import { server as WebSocketServer } from "websocket";
import * as Http from "http";
import * as WebSocket from "websocket";
import { logger } from "@vscode/debugadapter";
import {
  Hsp3DebugAdapterProtocol,
  Hsp3DebugAdapterPort,
} from "./constants";

/**
 * デバッガー (hsp3debug) と通信するための WebSocket サーバーを表す。
 */
export class GingerConnectionServer {
  private _httpServer: Http.Server;
  private _wsServer: WebSocketServer;
  private _connection: WebSocket.connection | undefined;
  private readonly _port: number = Hsp3DebugAdapterPort

  constructor(
    private readonly _handler: (message: string) => void,
  ) {
  }

  start() {
    this._httpServer = Http.createServer((request, response) => {
      logger.verbose('Received request for ' + request.url);
      response.writeHead(404);
      response.end();
    });

    this._httpServer.listen(this._port, () => {
      logger.verbose(`Listening on port ${this._port}`);
    });

    this._wsServer = new WebSocketServer({
      httpServer: this._httpServer,
    });

    this._wsServer.on("close", () => {
      logger.verbose("WebSocketServer 停止")
    });

    this._wsServer.on('request', request => {
      if (!this.originIsAllowed(request.origin)) {
        logger.verbose("WebSocketServer 不正なオリジンからのリクエストをブロックしました")
        return request.reject();
      }

      const connection = request.accept(Hsp3DebugAdapterProtocol, request.origin);

      connection.on('message', message => {
        if (!(message.type === 'utf8' && message.utf8Data !== undefined)) {
          logger.warn("WebSocketServer 受信 失敗 バイナリ")
          return;
        }

        logger.verbose("WebSocketServer 受信 " + message.utf8Data);

        this._handler(message.utf8Data);
      });

      connection.on('close', (reasonCode, description) => {
        logger.log('WebSocketServer 切断 ' + connection.remoteAddress);
        logger.verbose(JSON.stringify({ reasonCode, description }))
        this._connection = undefined;
      });

      this._connection = connection;
      logger.log("WebSocketServer 接続を確立しました");
    });
  }

  stopServer() {
  }

  send(message: string): void {
    const connection = this._connection;
    if (connection === undefined) return;

    connection.sendUTF(message);
    logger.verbose("WebSocketServer 送信 " + message)
  }

  private originIsAllowed(_origin) {
    return true;
  }
}
