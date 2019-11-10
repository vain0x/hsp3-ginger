// DLL アプリケーションのエントリポイント

#include "pch.h"

#define EXPORT extern "C" __declspec (dllexport)

using Hsp3MsgFn = void(*)(HSPCTX*);

using SetModeFn = void(*)(int);

using LogFn = void(*)(wchar_t const* data, std::size_t size);

// spider-server プロジェクトが生成する静的ライブラリで定義される。
extern "C" void spider_server_initialize(SetModeFn, LogFn log);
extern "C" void spider_server_terminate();
extern "C" void spider_server_logmes(char const* data, std::size_t size);

// assert, stop やステップ実行の完了により、HSP スクリプトの実行が一時停止したとき。
static auto const HSP3DEBUG_NOTICE_STOP = 0;

// logmes 命令が実行されたとき。ログの内容は HSP3CTX::stmp にあります。
static auto const HSP3DEBUG_NOTICE_LOGMES = 1;

static auto s_debug = (HSP3DEBUG*)nullptr;

BOOL APIENTRY DllMain(HMODULE instance, DWORD reason, LPVOID _reserved) {
	switch (reason) {
	case DLL_PROCESS_ATTACH:
#ifdef _DEBUG
		// Shift キーが押されているとき
		if (GetKeyState(VK_SHIFT) & 0x8000) {
			MessageBox(
				HWND{},
				TEXT("Ctrl+Alt+P でプロセス hsp3.exe にアタッチし、デバッグを開始してください。"),
				TEXT("hsp3debug"),
				MB_OK
			);
		}
#endif
		OutputDebugString(TEXT("hsp3debug attach\n"));
		break;
	case DLL_THREAD_ATTACH:
	case DLL_THREAD_DETACH:
		break;
	case DLL_PROCESS_DETACH:
		OutputDebugString(TEXT("hsp3debug detach\n"));
		spider_server_terminate();
		break;
	}
	return TRUE;
}

extern "C" void write_debug_log(wchar_t const* data, std::size_t size) {
	static auto buffer = std::vector<wchar_t>{};
	buffer.clear();
	buffer.insert(buffer.end(), data, data + size);
	buffer.push_back(0);

	OutputDebugString(data);
}

extern "C" void set_debug_mode(int debug_mode) {
	// FIXME: なるべくメインスレッド上で実行したい
	s_debug->dbg_set(debug_mode);
	
	// HSP のウィンドウにメッセージを送信することで、実行モードの変更に気づいてもらいます。
	// メッセージ自体に意味はありません。
	PostMessage(HWND_BROADCAST, WM_NULL, WPARAM{}, LPARAM{});
}

// デバッガーがアタッチされたときに HSP ランタイムから呼ばれます。
EXPORT BOOL APIENTRY debugini(HSP3DEBUG* debug, int _nouse1, int _nouse2, int _nouse3) {
	OutputDebugString(TEXT("debugini\n"));

	s_debug = debug;

	spider_server_initialize(set_debug_mode, write_debug_log);
	return 0;
}

// logmes や assert 0 が実行されたときに HSP ランタイムから呼ばれます。
EXPORT BOOL WINAPI debug_notice(HSP3DEBUG* debug, int reason, int _nouse1, int _nouse2) {
	switch (reason) {
	case HSP3DEBUG_NOTICE_STOP:
		OutputDebugString(TEXT("debug_notice (stop)\n"));
		break;

	case HSP3DEBUG_NOTICE_LOGMES: {
		auto data = debug->hspctx->stmp;

		OutputDebugString(TEXT("debug_notice (logmes): {\"\n"));
		OutputDebugStringA(data); // FIXME: 文字コード変換
		OutputDebugString(TEXT("\n\"}\n"));

		spider_server_logmes(data, std::strlen(data));
		break;
	}
	default:
		OutputDebugString(TEXT("debug_notice (unknown)\n"));
		break;
	}
	return 0;
}
