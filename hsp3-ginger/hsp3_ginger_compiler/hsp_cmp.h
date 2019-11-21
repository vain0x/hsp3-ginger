#ifndef HSP3_GINGER_COMPILER_HSP_CMP_
#define HSP3_GINGER_COMPILER_HSP_CMP_

#include <memory>
#include <string>
#include <Windows.h>

struct BMSCR;

struct ModuleCloseFn {
    void operator()(HMODULE h) const { FreeLibrary(h); }
};

using ModuleHandle = std::unique_ptr<std::remove_pointer<HMODULE>::type, ModuleCloseFn>;

struct HspCmpOptions {
    bool debug_mode;
    bool preprocess_only;
    bool output_utf8;
    bool no_hspdef;
    bool input_utf8;
    bool open_debug_window;

    void set_release_mode(bool release_mode);

    int to_compile_flags();
    int to_preprocessor_flags();
    int to_debug_window_flags();
};

class HspCmp {
    ModuleHandle handle_;

    // コンパイル処理を開始する。
    // p1: ソースコードのファイル名
    BOOL(WINAPI* hsc_ini)(BMSCR* bm, char* p1, int p2, int p3);

    // ソースコードの名前を指定する。この名前はエラーメッセージで使用される。
    // p1: 名前
    BOOL(WINAPI* hsc_refname)(BMSCR* bm, char* p1, int p2, int p3);

    // オブジェクトファイルのファイル名を指定する。
    // p1: ファイル名
    BOOL(WINAPI* hsc_objname)(BMSCR* bm, char* p1, int p2, int p3);

    // コンパイラのバージョンを表す文字列を取得する。
    // p4: 出力先のバッファー
    BOOL(WINAPI* hsc_ver)(int p1, int p2, int p3, char* p4);

    // コンパイル処理を終了する。
    BOOL(WINAPI* hsc_bye)(int p1, int p2, int p3, int p4);

    // コンパイラーからのメッセージを取得する。
    // p1: 出力先のバッファー
    BOOL(WINAPI* hsc_getmes)(char* p1, int p2, int p3, int p4);

    // コンパイラーのメッセージを消去する。
    BOOL(WINAPI* hsc_clrmes)(int p1, int p2, int p3, int p4);

    // `common` ディレクトリのパスを指定する。
    // p1: common ディレクトリへの絶対パス。末尾に `/` または `\` が必須。
    BOOL(WINAPI* hsc_compath)(BMSCR* bm, char* p1, int p2, int p3);

    // コンパイルを行う。
    // p1 = mode : 以下のビットフラグの組み合わせ
    //     1 デバッグモード
    //     2 プリプロセスのみ
    //     4 utf-8 出力モード
    // p2 = ppout : 以下のビットフラグの組み合わせ
    //     1 ver 2.6 モード
    //     4 utf-8 入力モード (スクリプトを utf-8 とみなして処理する)
    // p3 = dbgopt
    //     1 デバッグウィンドウを開いた状態で起動する。
    BOOL(WINAPI* hsc_comp)(int p1, int p2, int p3, int p4);

    // シンボル名を取得する。
    // FIXME: p1 の意味を調べる。
    BOOL(WINAPI* hsc3_getsym)(int p1, int p2, int p3, int p4);

    // メッセージの長さをバイト数で取得する。
    // p1: メッセージの長さを格納するアドレス
    BOOL(WINAPI* hsc3_messize)(int* p1, int p2, int p3, int p4);

    // 実行ファイルを生成する。
    // p1: ランタイムライブラリのディレクトリへのパス (`#runtime`
    // で指定した名前が連結される。) p2: iconins (リソースの書き換え) を行う。
    BOOL(WINAPI* hsc3_make)(BMSCR* bm, char* p1, int p2, int p3);

    // ランタイムの名前を取得する。
    // p1: ランタイム名を格納するアドレス。結果が "" のときは "hsp3.exe" を表す。
    // p2: オブジェクトファイルのパス
    BOOL(WINAPI* hsc3_getruntime)(char* p1, char* p2, int p3, int p4);

    // ソースコードを実行する。
    // p1: ランタイムを実行するコマンドライン
    BOOL(WINAPI* hsc3_run)(char* p1, int p2, int p3, int p4);

    void init(const char* src_file_path);
    void bye();

public:
    // dll_file_path: `hspcmp.dll` へのファイルパス。
    HspCmp(LPCTSTR dll_file_path, const char* src_file_path);
    HspCmp(HspCmp&&) = default;
    ~HspCmp();

    void set_ref_name(const char* ref_name);
    void set_obj_name(const char* obj_name);
    auto get_version()->std::string;
    auto get_message()->std::string;
    void clear_message();
    void set_common_path(const char* common_path);

    // return: 成功したら true
    bool compile(HspCmpOptions options);

    auto get_symbols()->std::string;
    auto get_runtime(const char* obj_name)->std::string;
    bool make(const char* runtime_dir);
    bool run(const char* command);
};

#endif
