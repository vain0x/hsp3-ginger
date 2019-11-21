// DLL アプリケーションのエントリポイント

#include "pch.h"
#include <array>
#include <cassert>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#define EXPORT extern "C" __declspec (dllexport)

#define WM_HDS_HELLO      (WM_USER +  0)
#define WM_HDS_SHUTDOWN   (WM_USER +  1)
#define WM_HDS_LOGMES     (WM_USER +  2)

// assert, stop やステップ実行の完了により、HSP スクリプトの実行が一時停止したとき。
static auto const HSP3DEBUG_NOTICE_STOP = 0;

// logmes 命令が実行されたとき。ログの内容は HSP3CTX::stmp にあります。
static auto const HSP3DEBUG_NOTICE_LOGMES = 1;

static auto const MAX_DATA_SIZE = 0x10000;

using OsString = std::basic_string<TCHAR>;
using OsStringView = std::basic_string_view<TCHAR>;

static auto s_instance = std::optional<HINSTANCE>{};
static auto s_debug = std::optional<HSP3DEBUG*>{};
static auto s_hds_client_hwnd = std::optional<HWND>{};
static auto s_hds_client_process = std::optional<HANDLE>{};
static auto s_hds_client_thread = std::optional<HANDLE>{};
static auto s_hds_pipe_name = OsString{ TEXT("\\\\.\\pipe\\hdspipe") };
static auto s_hds_pipe_server = std::optional<std::thread>{};
static auto s_hds_down_buffer_name = OsString{ TEXT("hdsdownbuffer") };
static auto s_hds_down_buffer_handle = std::optional<HANDLE>{};
static auto s_hds_down_buffer_view = std::optional<char*>{};
static auto s_terminated = false;

static auto get_hsp_dir() -> OsString {
	// DLL の絶対パスを取得する。
	auto buffer = std::array<TCHAR, MAX_PATH>{};
	GetModuleFileName(GetModuleHandle(nullptr), buffer.data(), buffer.size());
	auto full_path = OsString{ buffer.data() };

	// ファイル名の部分を削除
	while (!full_path.empty()) {
		auto last = full_path[full_path.length() - 1];
		if (last == TEXT('/') || last == TEXT('\\')) {
			break;
		}

		full_path.pop_back();
	}

	return full_path;
}

// -----------------------------------------------
// パイプ
//
// クライアントにメッセージを送信する。
// -----------------------------------------------

static auto serve_hds_pipe() -> int {
	static auto const MAX_INSTANCE_COUNT = 2;

	auto pipe_handle = CreateNamedPipe(
		s_hds_pipe_name.data(),
		PIPE_ACCESS_DUPLEX,
		PIPE_TYPE_BYTE | PIPE_WAIT,
		MAX_INSTANCE_COUNT,
		// out buffer size
		0,
		// in buffer size
		0,
		// deafult timeout
		100,
		NULL
	);
	if (pipe_handle == INVALID_HANDLE_VALUE) {
		OutputDebugString(TEXT("WARN: could not create hds_pipe"));
		return 1;
	}

	if (!ConnectNamedPipe(pipe_handle, NULL)) {
		OutputDebugString(TEXT("WARN: could not connect to hds_pipe"));
		CloseHandle(pipe_handle);
		return 1;
	}

	auto buffer = std::vector<char>{};
	buffer.resize(1024);
	auto read_size = DWORD{};

	while (true) {
		// まず "Content-Length: 数字\r\n\r\n" という文字列を読む。
		{
			read_size = DWORD{};
			if (!ReadFile(pipe_handle, buffer.data(), buffer.size(), &read_size, NULL)) {
				OutputDebugString(TEXT("WARN: cannot read header from pipe"));
				break;
			}
			if (buffer.size() <= read_size) {
				buffer.resize(read_size + 1);
			}
			buffer[read_size] = '\0';

			auto text = std::string_view{ buffer.data(), read_size };
			auto head = std::string_view{ u8"Content-Length: " };
			auto eol = text.find('\r');
			if (eol == std::string::npos || eol < head.size()) {
				OutputDebugString(TEXT("WARN: expected header"));
				continue;
			}
			buffer[eol] = '\0';
			auto body_size = (std::size_t)std::atol(text.data() + head.size());
			if (body_size >= MAX_DATA_SIZE) {
				continue;
			}

			if (buffer.size() <= body_size) {
				buffer.resize(body_size + 1);
			}
		}

		// 次に指定された長さのデータを読む。
		{
			read_size = DWORD{};
			if (!ReadFile(pipe_handle, buffer.data(), buffer.size(), &read_size, NULL)) {
				break;
			}
			if (buffer.size() <= read_size) {
				buffer.resize(read_size + 1);
			}
			buffer[read_size] = '\0';

			OutputDebugString(TEXT("body: "));
			OutputDebugStringA(buffer.data());
		}

		// 読み込んだデータに関して何らかの処理を行う。

		if (strncmp(buffer.data(), u8"hello:", 6) == 0) {
			OutputDebugString(TEXT("receive hello"));

			auto text = std::string_view{ buffer.data(), read_size };
			auto eol = text.find('\r');
			if (eol == std::string::npos) {
				OutputDebugString(TEXT("WARN: invalid hello"));
				continue;
			}
			buffer[eol] = '\0';

			auto hwnd = (HWND)atoll(buffer.data() + 6);

			assert(!s_hds_client_hwnd);
			s_hds_client_hwnd = hwnd;
			SendMessage(hwnd, WM_HDS_HELLO, WPARAM{}, LPARAM{});

		} else if (strncmp(buffer.data(), u8"shutdown", 8) == 0) {
			OutputDebugString(TEXT("receive shutdown"));
			break;
		} else {
			OutputDebugString(TEXT("WARN: unknown message"));
		}
	}

	FlushFileBuffers(pipe_handle);
	DisconnectNamedPipe(pipe_handle);
	CloseHandle(pipe_handle);
	return 0;
}

// -----------------------------------------------
// メモリマップトファイル
//
// クライアントにメッセージの内容を渡すための領域。
// -----------------------------------------------

static auto create_down_buffer() -> bool {
	static auto BUFFER_SIZE = 0x8000;
	auto handle = CreateFileMapping(
		INVALID_HANDLE_VALUE,
		NULL,
		PAGE_READWRITE,
		0,
		BUFFER_SIZE,
		s_hds_down_buffer_name.data()
	);
	if (!handle) {
		OutputDebugString(TEXT("could not create buffer"));
		return false;
	}
	s_hds_down_buffer_handle = handle;

	auto view = MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, 0);
	if (!view) {
		OutputDebugString(TEXT("Couldn't map buffer"));
		return false;
	}
	s_hds_down_buffer_view = reinterpret_cast<char*>(view);

	return true;
}

// -----------------------------------------------
// メインウィンドウ
// -----------------------------------------------

static LRESULT WINAPI main_window_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
	switch (msg) {
	//case WM_USER:
	//	OutputDebugString(TEXT("dbg_set"));
	//	(*s_debug)->dbg_set(HSPDEBUG_RUN);
	//	PostMessage(HWND_BROADCAST, WM_NULL, WPARAM{}, LPARAM{});
	//	break;

	case WM_CREATE:
		return TRUE;

	case WM_CLOSE:
		return FALSE;

	case WM_DESTROY:
		PostQuitMessage(0);
		break;

	default:
		break;
	}
	return DefWindowProc(hwnd, msg, wp, lp);
}

static auto create_main_window() -> HWND {
	static auto const WINDOW_CLASS_NAME = TEXT("HdsMainWindowClass");

	auto wndclass = WNDCLASS{};
	wndclass.style = CS_HREDRAW | CS_VREDRAW;
	wndclass.lpfnWndProc = main_window_proc;
	wndclass.cbClsExtra = 0;
	wndclass.cbWndExtra = 0;
	wndclass.hInstance = *s_instance;
	wndclass.hIcon = nullptr;
	wndclass.hCursor = LoadCursor(nullptr, IDC_ARROW);
	wndclass.hbrBackground = (HBRUSH)(COLOR_BTNFACE + 1);
	wndclass.lpszMenuName = nullptr;
	wndclass.lpszClassName = WINDOW_CLASS_NAME;
	RegisterClass(&wndclass);

	auto title = TEXT("hsp3-debug-self");
	auto style = WS_CAPTION | WS_VISIBLE | WS_THICKFRAME;
	auto size_x = 100;
	auto size_y = 100;
	auto pos_x = -size_x;
	auto pos_y = -size_y;
	auto hwnd = CreateWindow(WINDOW_CLASS_NAME, title, style, pos_x, pos_y, size_x, size_y, HWND{}, HMENU{}, *s_instance, LPARAM{});
	if (!hwnd) {
		MessageBox(HWND{}, TEXT("デバッグウィンドウの初期化に失敗しました。"), TEXT("hsp3-debug-self"), 0);
		exit(EXIT_FAILURE);
	}

	ShowWindow(hwnd, SW_HIDE);
	return hwnd;
}

// -----------------------------------------------
// アプリケーション
// -----------------------------------------------

static void app_initialize(HSP3DEBUG* debug) {
	s_debug = debug;

	s_hds_pipe_server = std::make_optional(std::thread{ [] { serve_hds_pipe(); } });

	if (!create_down_buffer()) {
		return;
	}

	// start client:
	auto name = get_hsp_dir();
	name += TEXT("hds_client_proxy.exe");

	auto cmdline = s_hds_pipe_name;

	auto si = STARTUPINFO{ sizeof(STARTUPINFO) };
	auto pi = PROCESS_INFORMATION{};

	auto success = CreateProcess(name.data(), cmdline.data(), NULL, NULL, FALSE, NORMAL_PRIORITY_CLASS, NULL, NULL, &si, &pi);
	if (!success) {
		OutputDebugString(TEXT("WARN: could not start child process"));
		return;
	}

	s_hds_client_process = std::make_optional(pi.hProcess);
	s_hds_client_thread = std::make_optional(pi.hThread);
}

static void app_terminate() {
	assert(!s_terminated);
	s_terminated = true;

	if (s_hds_client_hwnd) {
		SendMessage(*s_hds_client_hwnd, WM_HDS_SHUTDOWN, WPARAM{}, LPARAM{});
		s_hds_client_hwnd.reset();
	}
	if (s_hds_pipe_server) {
		s_hds_pipe_server->join();
		s_hds_pipe_server.reset();
	}
	if (s_hds_client_thread) {
		CloseHandle(*s_hds_client_thread);
		s_hds_client_thread.reset();
	}
	if (s_hds_client_process) {
		CloseHandle(*s_hds_client_process);
		s_hds_client_process.reset();
	}
}

static void app_logmes(char const* text) {
	// FIXME: hello までの logmes はバッファーに溜めておき、hello の時点で送信する

	if (s_terminated || !s_hds_client_hwnd || !s_hds_down_buffer_view) {
		return;
	}
	auto buffer = *s_hds_down_buffer_view;

	auto size = std::strlen(text);
	memcpy(buffer, text, size);
	buffer[size] = '\0';

	SendMessage(*s_hds_client_hwnd, WM_HDS_LOGMES, size, LPARAM{});
	return;
}

// -----------------------------------------------
// エントリーポイント
// -----------------------------------------------

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
		s_instance = instance;
		break;

	case DLL_THREAD_ATTACH:
	case DLL_THREAD_DETACH:
		break;

	case DLL_PROCESS_DETACH:
		OutputDebugString(TEXT("hsp3debug detach\n"));
		app_terminate();
		break;
	}
	return TRUE;
}

// デバッガーがアタッチされたときに HSP ランタイムから呼ばれます。
EXPORT BOOL APIENTRY debugini(HSP3DEBUG* debug, int _nouse1, int _nouse2, int _nouse3) {
	OutputDebugString(TEXT("debugini\n"));

	app_initialize(debug);
	return 0;
}

// logmes や assert 0 が実行されたときに HSP ランタイムから呼ばれます。
EXPORT BOOL WINAPI debug_notice(HSP3DEBUG* debug, int reason, int _nouse1, int _nouse2) {
	switch (reason) {
	case HSP3DEBUG_NOTICE_STOP:
		OutputDebugString(TEXT("debug_notice (stop)\n"));
		break;

	case HSP3DEBUG_NOTICE_LOGMES:
		OutputDebugString(TEXT("debug_notice (logmes): {\"\n"));
		OutputDebugStringA(debug->hspctx->stmp); // FIXME: 文字コード変換
		OutputDebugString(TEXT("\n\"}\n"));

		app_logmes(debug->hspctx->stmp);
		break;

	default:
		OutputDebugString(TEXT("debug_notice (unknown)\n"));
		break;
	}

	// 実行を再開します。(「実行」ボタンが押された時の処理。)
	debug->dbg_set(HSPDEBUG_RUN);

	// HSP のウィンドウにメッセージを送信することで、実行が再開されたことに気づいてもらいます。
	// メッセージ自体に意味はありません。
	PostMessage(HWND_BROADCAST, WM_NULL, WPARAM{}, LPARAM{});
	return 0;
}
