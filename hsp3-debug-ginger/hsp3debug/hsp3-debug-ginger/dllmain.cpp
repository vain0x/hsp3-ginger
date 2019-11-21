#include "stdafx.h"
#include "hspsdk/hsp3debug.h"
#include "hspsdk/hsp3struct.h"
#include <array>
#include <cassert>
#include <cctype>
#include <fstream>
#include <vector>

#define EXPORT extern "C" __declspec (dllexport)
#define BOOL int

typedef BOOL(WINAPI* DebugInitFn)(HSP3DEBUG* p1, int p2, int p3, int p4);
typedef BOOL(WINAPI* DebugNoticeFn)(HSP3DEBUG* p1, int p2, int p3, int p4);

// 文字列の末尾の空白を除去する。
static auto trim_end(std::string& s) {
    s.erase(std::find_if(s.rbegin(), s.rend(), [](char ch) {
        return !std::isspace(ch);
    }).base(), s.end());
}

// 文字列の末尾が suffix に一致するか判定する。
static auto ends_with(std::wstring const& str, std::wstring const& suffix) {
    if (str.length() < suffix.length()) {
        return false;
    }
    return std::equal(str.end() - suffix.length(), str.end(), suffix.begin(), suffix.end());
}

// 文字列の末尾から指定された文字列を除去する。
static auto strip_suffix(std::wstring const& str, std::wstring const& suffix) {
    assert(ends_with(str, suffix));
    return std::wstring{ str.begin(), str.end() - suffix.length() };
}

static auto utf8_to_wide_string(std::string const& str) {
    // バッファサイズを計算する。終端文字を含まないので注意。
    auto size = 1 + MultiByteToWideChar(CP_UTF8, 0, str.c_str(), str.length(), nullptr, 0);
    assert(size > 1);

    auto buf = std::vector<wchar_t>();
    buf.resize(size, L'\0');
    auto code = MultiByteToWideChar(CP_UTF8, 0, str.c_str(), str.length(), buf.data(), buf.size());
    assert(code != 0);

    return std::wstring{ buf.data() };
}

// エラーメッセージを出力して異常終了する。
[[noreturn]]
static auto fail_with(std::wstring const& str) -> void {
    MessageBox(nullptr, str.c_str(), L"hsp3debug", MB_OK);
    std::abort();
}

struct ModuleCloseFn {
    void operator()(HMODULE h) const {
        FreeLibrary(h);
    }
};

using ModuleHandle = std::unique_ptr<std::remove_pointer<HMODULE>::type, ModuleCloseFn>;

static auto load_library(std::wstring const& full_path) {
    auto handle = LoadLibrary(full_path.c_str());
    if (handle == nullptr) {
        fail_with(L"Couldn't load library " + full_path);
    }

    return ModuleHandle{ handle };
}

class DebugLibrary {
public:
    ModuleHandle handle;
    DebugInitFn debugini;
    DebugNoticeFn debug_notice;

    operator bool() {
        return !!handle;
    }
};

static auto load_debug_library(std::wstring const& full_path) {
    auto handle = load_library(full_path);
    auto debugini = (DebugInitFn)GetProcAddress(handle.get(), "debugini");
    auto debug_notice = (DebugInitFn)GetProcAddress(handle.get(), "debug_notice");
    return DebugLibrary{
        std::move(handle),
        debugini,
        debug_notice,
    };
}

// このモジュール(DLL)のフルパスからディレクトリ名の部分だけ取得する。(e.g. C:/hsp/)
static auto current_module_directory_name(HMODULE h_module) {
    // フルパスを取得する。
    auto buf = std::array<wchar_t, MAX_PATH>();
    GetModuleFileName(h_module, buf.data(), buf.size());
    auto full_path = std::wstring{ buf.data() };

    return strip_suffix(full_path, std::wstring{ L"hsp3debug.dll" });
}

static auto attach_debugger(std::wstring const& dir_name) -> DebugLibrary {
    auto full_path = dir_name + L"hsp3debug-ginger-adapter.dll";
    return load_debug_library(full_path);
}

static DebugLibrary s_lib{};

BOOL APIENTRY DllMain(
    HMODULE h_module,
    DWORD  ul_reason_for_call,
    LPVOID reserved
)
{
    switch (ul_reason_for_call)
    {
        case DLL_PROCESS_ATTACH: {
#ifndef NDEBUG
            MessageBox(nullptr, L"attach!", L"hsp3debug", MB_OK);
#endif
            auto dir_name = current_module_directory_name(h_module);
            auto lib = attach_debugger(dir_name);
            std::swap(s_lib, lib);
            break;
        }
        case DLL_THREAD_ATTACH:
        case DLL_THREAD_DETACH:
            break;
        case DLL_PROCESS_DETACH: {
            auto lib = DebugLibrary{};
            std::swap(s_lib, lib);
            break;
        }
    }
    return TRUE;
}

// デバッガーがアタッチされたときに HSP ランタイムから呼ばれる。
EXPORT BOOL APIENTRY debugini(HSP3DEBUG* p1, int p2, int p3, int p4)
{
    if (!s_lib || !s_lib.debug_notice) {
        fail_with(L"Couldn't not load debugini from the library.");
    }

    (s_lib.debugini)(p1, p2, p3, p4);
    return 0;
}

// logmes や assert 0 が実行されたときに HSP ランタイムから呼ばれる。
EXPORT BOOL WINAPI debug_notice(HSP3DEBUG* p1, int p2, int p3, int p4)
{
    if (!s_lib || !s_lib.debug_notice) {
        fail_with(L"Couldn't not load debug_notice from the library.");
    }

    (s_lib.debug_notice)(p1, p2, p3, p4);
    return 0;
}
