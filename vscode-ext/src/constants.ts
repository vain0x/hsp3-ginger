export const Hsp3DebugType = "hsp3"

/** adapter と接続するときのサブプロトコル */
// FIXME: ws-rs でサブプロトコルを指定する方法が分からないので、
// ひとまずプロトコルを限定せずに通信する。
export const Hsp3DebugAdapterProtocol = null as any

/** websocket 通信のポート */
export const Hsp3DebugAdapterPort = 8089
